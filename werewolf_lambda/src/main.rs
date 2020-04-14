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
struct WerewolfEvent {
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
    let event: WerewolfEvent = serde_json::from_str(&body).unwrap();
    
    let table_name = env::var("tableName").unwrap();

    let current_game = helpers::get_state(table_name, e.clone(), event.data.code.clone());
    match current_game {
        Some(item) => werewolf(e, item, event.data.player),
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

fn werewolf(event: types::ApiGatewayWebsocketProxyRequest, item: HashMap<String, AttributeValue>, eat_player_name: String) {
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
                if game_state.phase.name != types::PhaseName::Werewolf {
                    helpers::send_error("Not a valid transition!".to_string(),
                        event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
                }
                else if attr.role != types::PlayerRole::Werewolf {
                    helpers::send_error("You are not a werewolf!".to_string(),
                        event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
                }
                else {
                    let eat_player: Vec<types::Player> = game_state.players.clone().into_iter()
                        .filter(|p| p.name == eat_player_name).collect();
                    if eat_player.len() != 1 {
                        helpers::send_error("Invalid player to eat!".to_string(),
                            event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
                    }
                    else {
                        match eat_player[0].attributes.clone() {
                            None => {
                                helpers::send_error("Player has no attributes!".to_string(),
                                    event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
                            },
                            Some(attr) => {
                                if !attr.alive || attr.team != types::PlayerTeam::Good {
                                    helpers::send_error("Invalid player to eat!".to_string(),
                                        event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
                                }
                                else {
                                    let num_werewolves = game_state.players.clone().into_iter()
                                        .filter(|p| {
                                            p.attributes.as_ref().unwrap().role == types::PlayerRole::Werewolf &&
                                            p.attributes.as_ref().unwrap().alive
                                        }).collect::<Vec<types::Player>>().len();
                                    let mut new_phase = game_state.phase.clone();
                                    let mut new_players = game_state.players.clone();

                                    new_phase.data.insert(players[0].clone().name, eat_player[0].clone().name);

                                    if new_phase.data.len() == num_werewolves {
                                        let num_other_votes = new_phase.data.clone().into_iter()
                                            .filter(|(_, value)| value.clone() != eat_player_name).collect::<HashMap<String, String>>().len();
                                        if num_other_votes < 1 {
                                            new_players.retain(|p| p.name != eat_player_name);
                                            let mut new_eaten_player = eat_player[0].clone();
                                            let mut new_attributes = eat_player[0].attributes.clone().unwrap();
                                            new_attributes.alive = false;
                                            new_eaten_player.attributes = Some(new_attributes);
                                            new_players.push(new_eaten_player);
                                            if helpers::check_game_over(&new_players) {
                                                let mut new_phase_data = HashMap::new();
                                                new_phase_data.insert("winner".to_string(), format!("{:?}", types::PlayerTeam::Evil));
                                                new_phase = types::Phase {
                                                    name: types::PhaseName::End,
                                                    data: new_phase_data,
                                                };
                                            }
                                            else {
                                                new_phase = types::Phase {
                                                    name: types::PhaseName::Day,
                                                    data: HashMap::new(),
                                                };
                                            }
                                        }
                                    }

                                    game_state.phase = new_phase;
                                    game_state.players = new_players;
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
