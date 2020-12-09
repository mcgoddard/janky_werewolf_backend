use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use std::time::Instant;

use rand::Rng;
use lambda::Context;
use rusoto_dynamodb::{DynamoDbClient, DynamoDb, PutItemInput};
use futures::executor::block_on;

use crate::ActionError;
use crate::helpers::{get_state, update_state};

#[derive(Deserialize, Serialize, Clone)]
struct JoinEvent {
    action: String,
    data: EventData,
}

#[derive(Deserialize, Serialize, Clone)]
struct EventData {
    name: String,
    secret: String,
    code: Option<String>,
}

pub fn handle_join(e: common::ApiGatewayWebsocketProxyRequest, c: Context) -> Result<(), ActionError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let p: JoinEvent = serde_json::from_str(&body).unwrap();
    
    if p.data.name == "" {
        error!("Empty name in request {}", c.request_id);
        return Err(ActionError::new(&"Empty first name".to_string()));
    }
    else if p.data.secret == "" {
        error!("Empty secret in request {}", c.request_id);
        return Err(ActionError::new(&"Empty secret".to_string()));
    }
    match p.data.code {
        None => new_game(e, p.data.name, p.data.secret),
        Some(code) => join_game(e, p.data.name, p.data.secret, code),
    }
}

fn new_game(event: common::ApiGatewayWebsocketProxyRequest, name: String, secret: String) -> Result<(), ActionError> {
    let table_name = env::var("tableName").unwrap();

    let code = create_random_code();

    let since_the_epoch = SystemTime::now().duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let ttl = (since_the_epoch.as_secs() as u32) + (48*60*60);

    let game_state = common::GameState {
        lobby_id: code,
        phase: common::Phase {
            name: common::PhaseName::Lobby,
            data: HashMap::new(),
        },
        players: vec![common::Player{
            id: event.request_context.connection_id.unwrap(),
            name,
            secret,
            attributes: common::PlayerAttributes {
                role: common::PlayerRole::Unknown,
                team: common::PlayerTeam::Unknown,
                alive: true,
                visible_to: vec!["All".to_string()],
            },
        }],
        internal_state: HashMap::new(),
        version: 1,
        ttl,
    };
    let item = serde_dynamodb::to_hashmap(&game_state);
    match item {
        Ok(item) => {
            let client = DynamoDbClient::new(Default::default());
            let result = block_on(client.put_item(PutItemInput {
                table_name,
                condition_expression: Some("attribute_not_exists(lobby_id)".to_string()),
                item,
                ..PutItemInput::default()
            }));
        
            match result {
                Ok(_) => Ok(()),
                Err(err) => {
                    error!("Failed to perform new game connection operation: {:?}", err);
                    Err(ActionError::new(&"Error creating game, please try again".to_string()))
                },
            }
        },
        Err(err) => {
            error!("Failed to create new game state: {:?}", err);
            Err(ActionError::new(&"Error creating game, please try again".to_string()))
        }
    }
}

fn join_game(event: common::ApiGatewayWebsocketProxyRequest, name: String, secret: String, lobby_id: String) -> Result<(), ActionError> {
    let table_name = env::var("tableName").unwrap();

    let start = Instant::now();
    let item = get_state(table_name.clone(), lobby_id);
    let duration = start.elapsed();
    println!("Time elapsed getting game state is: {:?}", duration);

    if let Ok(item) = item {
        let mut data: common::GameState = item;
        let existing_player: Vec<common::Player> = data.players.clone().into_iter().filter(|player| player.name == name).collect();
        if existing_player.len() == 1 {
            if existing_player[0].secret == secret {
                let mut new_players = data.players.clone();
                new_players.retain(|player| player.name != name);
                new_players.push(common::Player{
                    id: event.request_context.connection_id.unwrap(),
                    name,
                    secret,
                    attributes: existing_player[0].attributes.clone(),
                });
                data.players = new_players;
            }
            else {
                error!("Non-matching secret for {:?}", name);
            }
        }
        else if data.phase.name == common::PhaseName::Lobby {
            data.players.push(common::Player{
                id: event.request_context.connection_id.unwrap(),
                name,
                secret,
                attributes: common::PlayerAttributes {
                    role: common::PlayerRole::Unknown,
                    team: common::PlayerTeam::Unknown,
                    alive: true,
                    visible_to: vec!["All".to_string()],
                },
            });
        }
        else {
            return Err(ActionError::new(&"Error cannot join an in-progress game".to_string()))
        }
        let start = Instant::now();
        let result = update_state(data, table_name);
        let duration = start.elapsed();
        println!("Time elapsed putting game state is: {:?}", duration);
        result
    } else {
        Err(ActionError::new(&"Game not found".to_string()))
    }
}

fn create_random_code() -> String {
    let mut rng = rand::thread_rng();
    let valid_code_chars = vec!["A","B","C","D","E","F","G","H","I","J","K","L","M","N","O","P","Q","R","S","T","U","V","W","X","Y","Z"];
    (0..4).map(|_| (valid_code_chars[rng.gen_range(0, 26) as usize]).to_owned()).collect()
}
