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
        DynamoDbClient, AttributeValue,
    },
};
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
struct SleepEvent {
    action: String,
    data: EventData,
}

#[derive(Deserialize, Serialize, Clone)]
struct EventData {
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
    let event: SleepEvent = serde_json::from_str(&body).unwrap();
    
    let table_name = env::var("tableName").unwrap();

    let current_game = helpers::get_state(table_name, e.clone(), event.data.code);
    match current_game {
        Some(item) => move_to_sleep(e, item),
        None => (),
    };

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: HashMap::new(),
        multi_value_headers: HashMap::new(),
        body: None,
        is_base64_encoded: None,
    })
}

fn move_to_sleep(event: types::ApiGatewayWebsocketProxyRequest, item: HashMap<String, AttributeValue>) {
    let table_name = env::var("tableName").unwrap();

    let mut game_state: types::GameState = serde_json::from_str(&item["data"].s.clone().unwrap()).unwrap();

    let players: Vec<types::Player> = game_state.players.clone().into_iter().filter(|p| p.id == event.request_context.connection_id.clone().unwrap()).collect();
    if players.len() != 1 {
        helpers::send_error(format!("Could not find player with connection ID: {:?}", event.request_context.connection_id.clone().unwrap()),
                event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
    }
    else {
        match &players[0].attributes {
            None => {
                helpers::send_error(format!("No attributes for player: {:?}", players[0].name),
                    event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
            }
            Some(attr) => {
                if game_state.phase.name != types::PhaseName::Day {
                    helpers::send_error("Not a valid transition!".to_string(),
                        event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
                }
                else if attr.role != types::PlayerRole::Mod {
                    helpers::send_error("You are not the moderator!".to_string(),
                        event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
                }
                else {
                    let seer_alive = game_state.players.clone().into_iter()
                        .filter(|p| p.attributes.as_ref().unwrap().role == types::PlayerRole::Seer && p.attributes.as_ref().unwrap().alive)
                        .collect::<Vec<types::Player>>().len();
                    match seer_alive {
                        1 => {
                            game_state.phase = types::Phase {
                                name: types::PhaseName::Seer,
                                data: HashMap::new(),
                            };
                        },
                        _ => {
                            game_state.phase = types::Phase {
                                name: types::PhaseName::Werewolf,
                                data: HashMap::new(),
                            };
                        },
                    };
                    helpers::update_state(item, game_state, table_name, event);
                }
            }
        }
    }
}
