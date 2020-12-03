use lambda::error::HandlerError;

use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;

use dynomite::{
    dynamodb::{
        DynamoDb, PutItemInput, AttributeValue,
    },
};
use rand::Rng;
use serde_json::json;
use futures::future::Future;

use common::{types, helpers};

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

pub fn handle_join(e: types::ApiGatewayWebsocketProxyRequest, c: lambda::Context) -> Result<ApiGatewayProxyResponse, HandlerError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let p: JoinEvent = serde_json::from_str(&body).unwrap();
    
    if p.data.name == "" {
        error!("Empty name in request {}", c.aws_request_id);
        helpers::send_error("Empty first name".to_string(),
            e.request_context.connection_id.clone().unwrap(), helpers::endpoint(&e.request_context));
    }
    else if p.data.secret == "" {
        error!("Empty secret in request {}", c.aws_request_id);
        helpers::send_error("Empty secret".to_string(),
            e.request_context.connection_id.clone().unwrap(), helpers::endpoint(&e.request_context));
    }
    else {
        match p.data.code {
            None => new_game(e, p.data.name, p.data.secret),
            Some(c) => join_game(e, p.data.name, p.data.secret, c),
        };
    }

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: HashMap::new(),
        multi_value_headers: HashMap::new(),
        body: None,
        is_base64_encoded: None,
    })
}

fn new_game(event: types::ApiGatewayWebsocketProxyRequest, name: String, secret: String) {
    let table_name = env::var("tableName").unwrap();

    let mut rng = rand::thread_rng();
    let valid_code_chars = vec!["A","B","C","D","E","F","G","H","I","J","K","L","M","N","O","P","Q","R","S","T","U","V","W","X","Y","Z"];
    let code: String = (0..4).map(|_| (valid_code_chars[rng.gen_range(0, 26) as usize]).to_owned()).collect();

    let item = types::GameState {
        lobby_id: code,
        phase: types::Phase {
            name: types::PhaseName::Lobby,
            data: HashMap::new(),
        },
        players: vec![types::Player{
            id: event.request_context.connection_id.clone().unwrap(),
            name,
            secret,
            attributes: types::PlayerAttributes {
                role: types::PlayerRole::Unknown,
                team: types::PlayerTeam::Unknown,
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
    let result = helpers::DDB.with(|ddb| {
        ddb.put_item(PutItemInput {
            table_name,
            condition_expression: Some("attribute_not_exists(lobby_id)".to_string()),
            item: item_hashmap,
            ..PutItemInput::default()
        })
        .map(drop)
        .map_err(types::RequestError::Connect)
    });

    if let Err(err) = helpers::RT.with(|rt| rt.borrow_mut().block_on(result)) {
        log::error!("failed to perform new game connection operation: {:?}", err);
        helpers::send_error(format!("Error creating game: {:?}", err),
            event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
    }
}

fn join_game(event: types::ApiGatewayWebsocketProxyRequest, name: String, secret: String, lobby_id: String) {
    let table_name = env::var("tableName").unwrap();

    let item = helpers::get_state(table_name.clone(), event.clone(), lobby_id);

    if let Some(item) = item {
        let mut data: types::GameState = serde_json::from_str(&item["data"].s.clone().unwrap()).unwrap();
        let existing_player: Vec<types::Player> = data.players.clone().into_iter().filter(|player| player.name == name).collect();
        if existing_player.len() == 1 {
            if existing_player[0].secret == secret {
                let mut new_players = data.players.clone();
                new_players.retain(|player| player.name != name);
                new_players.push(types::Player{
                    id: event.request_context.connection_id.clone().unwrap(),
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
        else if data.phase.name == types::PhaseName::Lobby {
            data.players.push(types::Player{
                id: event.request_context.connection_id.clone().unwrap(),
                name,
                secret,
                attributes: types::PlayerAttributes {
                    role: types::PlayerRole::Unknown,
                    team: types::PlayerTeam::Unknown,
                    alive: true,
                    visible_to: vec!["All".to_string()],
                },
            });
        }
        else {
            helpers::send_error("Error cannot join an in-progress game".to_string(),
                event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
        }
        helpers::update_state(item, data, table_name, event);
    }
}
