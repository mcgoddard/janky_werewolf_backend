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

use common::{types, helpers};

thread_local!(
    static DDB: DynamoDbClient = DynamoDbClient::new(Default::default());
);

thread_local!(
    static RT: RefCell<Runtime> =
        RefCell::new(Runtime::new().expect("failed to initialize runtime"));
);

#[derive(Deserialize, Serialize, Clone)]
struct LynchEvent {
    action: String,
    data: EventData,
}

#[derive(Deserialize, Serialize, Clone)]
struct EventData {
    code: String,
    player: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::init_with_level(log::Level::Info)?;
    lambda!(my_handler);

    Ok(())
}

fn my_handler(e: types::ApiGatewayWebsocketProxyRequest, _c: lambda::Context) -> Result<ApiGatewayProxyResponse, HandlerError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let event: LynchEvent = serde_json::from_str(&body).unwrap();
    
    let table_name = env::var("tableName").unwrap();

    let current_game = helpers::get_state(table_name, e.clone(), event.data.code);
    if let Some(item) = current_game { move_to_sleep(e, item, event.data.player) }

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: HashMap::new(),
        multi_value_headers: HashMap::new(),
        body: None,
        is_base64_encoded: None,
    })
}

fn move_to_sleep(event: types::ApiGatewayWebsocketProxyRequest, item: HashMap<String, AttributeValue>, lynched_player: String) {
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
                    let killing_player: Vec<types::Player> = game_state.players.clone().into_iter()
                        .filter(|p| p.name == lynched_player).collect();
                    if killing_player.len() != 1 {
                        helpers::send_error("Invalid player to lynch!".to_string(),
                            event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
                    }
                    else {
                        match killing_player[0].attributes.clone() {
                            None => {
                                helpers::send_error("Player has no attributes!".to_string(),
                                    event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
                            },
                            Some(attr) => {
                                if !attr.alive {
                                    helpers::send_error("Player is already dead!".to_string(),
                                        event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
                                }
                                else {
                                    let mut new_players = game_state.players.clone();
                                    new_players.retain(|p| p.name != lynched_player);
                                    let mut new_attributes = killing_player[0].attributes.clone().unwrap();
                                    new_attributes.alive = false;
                                    let mut new_killing_player = killing_player[0].clone();
                                    new_killing_player.attributes = killing_player[0].attributes.clone();
                                    new_killing_player.attributes = Some(new_attributes);
                                    new_players.push(new_killing_player);
                                    if helpers::check_game_over(new_players.clone()) {
                                        let mut new_phase_data = HashMap::new();
                                        let winner: types::PlayerTeam;
                                        match new_players.clone().into_iter().filter(|p| p.attributes.as_ref().unwrap().team == types::PlayerTeam::Evil && p.attributes.as_ref().unwrap().alive)
                                            .count() {
                                            0 => {
                                                winner = types::PlayerTeam::Good;
                                            },
                                            _ => {
                                                winner = types::PlayerTeam::Evil;
                                            }
                                        };
                                        game_state.players = new_players;
                                        new_phase_data.insert("winner".to_string(), format!("{:?}", winner));
                                        game_state.phase = types::Phase {
                                            name: types::PhaseName::End,
                                            data: new_phase_data,
                                        };
                                    }
                                    else {
                                        game_state.players = new_players;
                                        game_state.phase = types::Phase {
                                            name: types::PhaseName::Seer,
                                            data: HashMap::new(),
                                        };
                                        let seer_alive = game_state.players.clone().into_iter()
                                            .filter(|p| p.attributes.as_ref().unwrap().role == types::PlayerRole::Seer && p.attributes.as_ref().unwrap().alive)
                                            .count();
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
                                    }
                                    helpers::update_state(item, game_state, table_name, event);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
