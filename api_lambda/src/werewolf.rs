use lambda::error::HandlerError;

use std::env;
use std::collections::HashMap;

use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;

use dynomite::{
    dynamodb::{
        AttributeValue,
    },
};

use crate::helpers::{get_state, send_error, endpoint, update_state, living_players_with_role, check_game_over};

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

pub fn handle_werewolf(e: common::ApiGatewayWebsocketProxyRequest) -> Result<ApiGatewayProxyResponse, HandlerError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let event: WerewolfEvent = serde_json::from_str(&body).unwrap();
    
    let table_name = env::var("tableName").unwrap();

    let current_game = get_state(table_name, e.clone(), event.data.code.clone());
    if let Some(item) = current_game { werewolf(e, item, event.data.player) }

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: HashMap::new(),
        multi_value_headers: HashMap::new(),
        body: None,
        is_base64_encoded: None,
    })
}

fn werewolf(event: common::ApiGatewayWebsocketProxyRequest, item: HashMap<String, AttributeValue>, eat_player_name: String) {
    let table_name = env::var("tableName").unwrap();

    let mut game_state: common::GameState = serde_json::from_str(&item["data"].s.clone().unwrap()).unwrap();

    let players: Vec<common::Player> = game_state.players.clone().into_iter().filter(|p| p.id == event.request_context.connection_id.clone().unwrap()).collect();
    if players.len() != 1 {
        send_error(format!("Could not find player with connection ID: {:?}", event.request_context.connection_id.clone().unwrap()),
                event.request_context.connection_id.clone().unwrap(), endpoint(&event.request_context));
    }
    else if game_state.phase.name != common::PhaseName::Werewolf {
        send_error("Not a valid transition!".to_string(),
            event.request_context.connection_id.clone().unwrap(), endpoint(&event.request_context));
    }
    else if players[0].attributes.role != common::PlayerRole::Werewolf {
        send_error("You are not a werewolf!".to_string(),
            event.request_context.connection_id.clone().unwrap(), endpoint(&event.request_context));
    }
    else {
        let eat_player: Vec<common::Player> = game_state.players.clone().into_iter()
            .filter(|p| p.name == eat_player_name).collect();
        if eat_player.len() != 1 || !eat_player[0].attributes.alive || eat_player[0].attributes.team != common::PlayerTeam::Good {
            send_error("Invalid player to eat!".to_string(),
                event.request_context.connection_id.clone().unwrap(), endpoint(&event.request_context));
        }
        else {
            let num_werewolves = game_state.players.clone().into_iter()
                .filter(|p| {
                    p.attributes.role == common::PlayerRole::Werewolf &&
                    p.attributes.alive
                }).count();
            let mut new_phase = game_state.phase.clone();
            let mut new_players = game_state.players.clone();

            new_phase.data.insert(players[0].clone().name, eat_player[0].clone().name);

            if new_phase.data.len() == num_werewolves {
                let num_other_votes = new_phase.data.clone().into_iter()
                    .filter(|(_, value)| value.clone() != eat_player_name).count();
                if num_other_votes < 1 {
                    let last_protected_player = game_state.internal_state.get("last_guarded").unwrap_or(&"".to_string()).clone();
                    if last_protected_player == eat_player_name &&
                        living_players_with_role(common::PlayerRole::Bodyguard, game_state.players) > 0 {
                            new_phase = common::Phase {
                                name: common::PhaseName::Day,
                                data: HashMap::new(),
                            };
                    }
                    else {
                        new_players.retain(|p| p.name != eat_player_name);
                        let mut new_eaten_player = eat_player[0].clone();
                        let mut new_attributes = eat_player[0].attributes.clone();
                        new_attributes.alive = false;
                        new_eaten_player.attributes = new_attributes;
                        new_players.push(new_eaten_player);
                        match check_game_over(new_players.clone()) {
                            Some(winners) => {
                                let mut new_phase_data = HashMap::new();
                                match winners.len() {
                                    1 => new_phase_data.insert("winner".to_string(), format!("{:?}", winners[0])),
                                    _ => new_phase_data.insert("winner".to_string(), winners.into_iter().map(|w| format!("{:?}", w)).collect::<Vec<String>>().join(", "))
                                };
                                
                                new_phase = common::Phase {
                                    name: common::PhaseName::End,
                                    data: new_phase_data,
                                };
                            },
                            None => {
                                new_phase = common::Phase {
                                    name: common::PhaseName::Day,
                                    data: HashMap::new(),
                                };
                            },
                        };
                    }
                }
            }

            game_state.phase = new_phase;
            game_state.players = new_players;
            update_state(item, game_state, table_name, event);
        }
    }
}
