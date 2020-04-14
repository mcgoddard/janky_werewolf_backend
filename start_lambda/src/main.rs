#[macro_use]
extern crate lambda_runtime as lambda;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate simple_logger;
extern crate rand;

use lambda::error::HandlerError;

use std;
use std::{cell::RefCell, env};
use std::error::Error;
use std::collections::HashMap;

use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;

use dynomite::{
    dynamodb::{
        DynamoDb, DynamoDbClient, AttributeValue, GetItemInput, GetItemOutput,
    },
};
use futures::Future;
use rand::Rng;
use tokio::runtime::Runtime;

mod types;
mod helpers;

thread_local!(
    static DDB: DynamoDbClient = DynamoDbClient::new(Default::default());
);

thread_local!(
    static RT: RefCell<Runtime> =
        RefCell::new(Runtime::new().expect("failed to initialize runtime"));
);

#[derive(Deserialize, Serialize, Clone)]
struct StartEvent {
    action: String,
    data: EventData,
}

#[derive(Deserialize, Serialize, Clone)]
struct EventData {
    werewolves: i32,
    code: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::init_with_level(log::Level::Info)?;
    lambda!(my_handler);

    Ok(())
}

fn my_handler(e: types::ApiGatewayWebsocketProxyRequest, _c: lambda::Context) -> Result<ApiGatewayProxyResponse, HandlerError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let event: StartEvent = serde_json::from_str(&body).unwrap();
    
    let table_name = env::var("tableName").unwrap();

    let mut ddb_keys = HashMap::new();
    ddb_keys.insert("lobby_id".to_string(), AttributeValue {
        s: Some(event.data.code.clone()),
        ..Default::default()
    });

    let result = DDB.with(|ddb| {
        ddb.get_item(GetItemInput {
            table_name: table_name.clone(),
            key: ddb_keys,
            ..GetItemInput::default()
        })
        .map(types::RequestResult::Get)
        .map_err(types::RequestError::Get)
    });

    match RT.with(|rt| rt.borrow_mut().block_on(result)) {
        Err(err) => {
            log::error!("failed to perform new game connection operation: {:?}", err);
            helpers::send_error(format!("Lobby not found: {:?}", err),
                e.request_context.connection_id.clone().unwrap(), helpers::endpoint(&e.request_context));
        },
        Ok(result) => {
            match result {
                types::RequestResult::Get(result) => {
                    let result: GetItemOutput = result;
                    match result.item {
                        None => {
                            error!("Lobby not found: {:?}", event.data.code);
                            helpers::send_error("Unable to find lobby".to_string(),
                                e.request_context.connection_id.clone().unwrap(), helpers::endpoint(&e.request_context));
                        },
                        Some(item) => {
                            move_to_day(e, item, event.data.werewolves);
                        },
                    }
                }
            }
        }
    }

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: HashMap::new(),
        multi_value_headers: HashMap::new(),
        body: None,
        is_base64_encoded: None,
    })
}

fn move_to_day(event: types::ApiGatewayWebsocketProxyRequest, item: HashMap<String, AttributeValue>, werewolves: i32) {
    let table_name = env::var("tableName").unwrap();

    let mut game_state: types::GameState = serde_json::from_str(&item["data"].s.clone().unwrap()).unwrap();
    
    let mut roles: Vec<types::PlayerAttributes> = vec![];
    let num_villagers = (game_state.players.len() as i32 - werewolves) - 2;
    roles.push(types::PlayerAttributes {
        role: types::PlayerRole::Seer,
        team: types::PlayerTeam::Good,
        alive: true,
        visible_to: vec![format!("{:?}", types::PlayerRole::Mod)],
    });
    for _ in 0..werewolves {
        roles.push(types::PlayerAttributes {
            role: types::PlayerRole::Werewolf,
            team: types::PlayerTeam::Evil,
            alive: true,
            visible_to: vec![format!("{:?}", types::PlayerRole::Mod), format!("{:?}", types::PlayerRole::Werewolf)],
        });
    }
    for _ in 0..num_villagers {
        roles.push(types::PlayerAttributes {
            role: types::PlayerRole::Villager,
            team: types::PlayerTeam::Good,
            alive: true,
            visible_to: vec![format!("{:?}", types::PlayerRole::Mod)],
        });
    }


    let mut new_players = vec![];
    let mut rng = rand::thread_rng();
    for player in &game_state.players {
        let mut new_player = player.clone();
        if player.id == event.request_context.connection_id.clone().unwrap() {
            new_player.attributes = Some(types::PlayerAttributes {
                role: types::PlayerRole::Mod,
                team: types::PlayerTeam::Unknown,
                alive: true,
                visible_to: vec!["All".to_string()],
            });
        }
        else {
            let role = roles.remove(rng.gen_range(0, roles.len()));
            new_player.attributes = Some(role);
        }
        new_players.push(new_player);
    }

    game_state.players = new_players;
    game_state.phase = types::Phase {
        name: types::PhaseName::Day,
        data: HashMap::new(),
    };

    helpers::update_state(item, game_state, table_name, event);
}
