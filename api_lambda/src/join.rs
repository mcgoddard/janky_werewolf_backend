use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

use dynomite::{
    dynamodb::{
        DynamoDb, PutItemInput, AttributeValue,
    },
};
use rand::Rng;
use serde_json::json;
use futures::future::Future;

use crate::ActionError;
use crate::helpers::{get_state, update_state, DDB, RT, RequestError};

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

pub fn handle_join(e: common::ApiGatewayWebsocketProxyRequest, c: lambda::Context) -> Result<(), ActionError> {
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
        None => new_game(e, p.data.name, p.data.secret),
        Some(c) => join_game(e, p.data.name, p.data.secret, c),
    }
}

fn new_game(event: common::ApiGatewayWebsocketProxyRequest, name: String, secret: String) -> Result<(), ActionError> {
    let table_name = env::var("tableName").unwrap();

    let mut rng = rand::thread_rng();
    let valid_code_chars = vec!["A","B","C","D","E","F","G","H","I","J","K","L","M","N","O","P","Q","R","S","T","U","V","W","X","Y","Z"];
    let code: String = (0..4).map(|_| (valid_code_chars[rng.gen_range(0, 26) as usize]).to_owned()).collect();

    let item = common::GameState {
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
    };
    let mut item_hashmap = HashMap::new();
    item_hashmap.insert("lobby_id".to_string(), AttributeValue {
        s: Some(item.lobby_id.clone()),
        ..Default::default()
    });
    item_hashmap.insert("version".to_string(), AttributeValue {
        n: Some(1.to_string()),
        ..Default::default()
    });
    let data = json!(item);
    item_hashmap.insert("data".to_string(), AttributeValue {
        s: Some(data.to_string()),
        ..Default::default()
    });
    let since_the_epoch = SystemTime::now().duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    item_hashmap.insert("ttl".to_string(), AttributeValue {
        n: Some(format!("{}", (since_the_epoch.as_secs() as i32) + (48*60*60))),
        ..Default::default()
    });
    let result = DDB.with(|ddb| {
        ddb.put_item(PutItemInput {
            table_name,
            condition_expression: Some("attribute_not_exists(lobby_id)".to_string()),
            item: item_hashmap,
            ..PutItemInput::default()
        })
        .map(drop)
        .map_err(RequestError::Connect)
    });

    if let Err(err) = RT.with(|rt| rt.borrow_mut().block_on(result)) {
        log::error!("failed to perform new game connection operation: {:?}", err);
        return Err(ActionError::new(&format!("Error creating game: {:?}", err)))
    }

    Ok(())
}

fn join_game(event: common::ApiGatewayWebsocketProxyRequest, name: String, secret: String, lobby_id: String) -> Result<(), ActionError> {
    let table_name = env::var("tableName").unwrap();

    let item = get_state(table_name.clone(), event.clone(), lobby_id);

    if let Some(item) = item {
        let mut data: common::GameState = serde_json::from_str(&item["data"].s.clone().unwrap()).unwrap();
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
        update_state(item, data, table_name)
    } else {
        Err(ActionError::new(&"Game not found".to_string()))
    }
}
