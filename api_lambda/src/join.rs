use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

use dynomite::{
    dynamodb::{
        DynamoDb, PutItemInput, DynamoDbClient
    },
};
use rand::Rng;

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

pub async fn handle_join(e: common::ApiGatewayWebsocketProxyRequest, c: lambda::Context) -> Result<(), ActionError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let p: JoinEvent = serde_json::from_str(&body).unwrap();
    
    if p.data.name == "" {
        error!("Empty name in request {}", c.aws_request_id);
        return Err(ActionError::new(&"Empty first name".to_string()));
    }
    else if p.data.secret == "" {
        error!("Empty secret in request {}", c.aws_request_id);
        return Err(ActionError::new(&"Empty secret".to_string()));
    }
    match p.data.code {
        None => new_game(e, p.data.name, p.data.secret).await,
        Some(code) => join_game(e, p.data.name, p.data.secret, code).await,
    }
}

async fn new_game(event: common::ApiGatewayWebsocketProxyRequest, name: String, secret: String) -> Result<(), ActionError> {
    let table_name = env::var("tableName").unwrap();

    let mut rng = rand::thread_rng();
    let valid_code_chars = vec!["A","B","C","D","E","F","G","H","I","J","K","L","M","N","O","P","Q","R","S","T","U","V","W","X","Y","Z"];
    let code: String = (0..4).map(|_| (valid_code_chars[rng.gen_range(0, 26) as usize]).to_owned()).collect();

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
        ttl: ttl,
    };
    let ddb = DynamoDbClient::new(Default::default());
    let result = ddb.put_item(PutItemInput {
            table_name,
            condition_expression: Some("attribute_not_exists(lobby_id)".to_string()),
            item: game_state.into(),
            ..PutItemInput::default()
        }).await;

    match result {
        Ok(_) => Ok(()),
        Err(err) => {
            error!("Failed to perform new game connection operation: {:?}", err);
            Err(ActionError::new(&"Error creating game, please try again".to_string()))
        },
    }
}

async fn join_game(event: common::ApiGatewayWebsocketProxyRequest, name: String, secret: String, lobby_id: String) -> Result<(), ActionError> {
    let table_name = env::var("tableName").unwrap();

    let item = get_state(table_name.clone(), lobby_id).await;

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
        update_state(data, table_name).await
    } else {
        Err(ActionError::new(&"Game not found".to_string()))
    }
}
