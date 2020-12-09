use std::env;
use std::collections::HashMap;

use crate::ActionError;
use crate::helpers::{get_state, update_state, check_game_over, living_players_with_role};

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

pub fn handle_lynch(e: common::ApiGatewayWebsocketProxyRequest) -> Result<(), ActionError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let event: LynchEvent = serde_json::from_str(&body).unwrap();
    
    let table_name = env::var("tableName").unwrap();

    let current_game = get_state(table_name, event.data.code);
    if let Ok(item) = current_game { 
        move_to_sleep(e, item, event.data.player)
    } else {
        Err(ActionError::new(&"Game not found".to_string()))
    }
}

fn move_to_sleep(event: common::ApiGatewayWebsocketProxyRequest, mut game_state: common::GameState, lynched_player: String)
        -> Result<(), ActionError> {
    let table_name = env::var("tableName").unwrap();

    let players: Vec<common::Player> = game_state.players.clone().into_iter().filter(|p| p.id == event.request_context.connection_id.clone().unwrap()).collect();
    if players.len() != 1 {
        return Err(ActionError::new(&format!("Could not find player with connection ID: {:?}", event.request_context.connection_id.unwrap())));
    }
    else if game_state.phase.name != common::PhaseName::Day {
        return Err(ActionError::new(&"Not a valid transition!".to_string()));
    }
    else if players[0].attributes.role != common::PlayerRole::Mod {
        return Err(ActionError::new(&"You are not the moderator!".to_string()));
    }
    let killing_player: Vec<common::Player> = game_state.players.clone().into_iter()
        .filter(|p| p.name == lynched_player).collect();
    if killing_player.len() != 1 {
        return Err(ActionError::new(&"Invalid player to lynch!".to_string()));
    }
    else if !players[0].attributes.alive {
        return Err(ActionError::new(&"Player is already dead!".to_string()));
    }
    let mut new_players = game_state.players.clone();
    new_players.retain(|p| p.name != lynched_player);
    let mut new_attributes = killing_player[0].attributes.clone();
    new_attributes.alive = false;
    let mut new_killing_player = killing_player[0].clone();
    new_killing_player.attributes = killing_player[0].attributes.clone();
    new_killing_player.attributes = new_attributes;
    new_players.push(new_killing_player);
    match check_game_over(new_players.clone()) {
        Some(winners) => {
            let mut new_phase_data = HashMap::new();
            game_state.players = new_players;
            match winners.len() {
                1 => new_phase_data.insert("winner".to_string(), format!("{:?}", winners[0])),
                _ => new_phase_data.insert("winner".to_string(), winners.into_iter().map(|w| format!("{:?}", w)).collect::<Vec<String>>().join(", ")),
            };
            
            game_state.phase = common::Phase {
                name: common::PhaseName::End,
                data: new_phase_data,
            };
        },
        None => {
            game_state.players = new_players;
            game_state.phase = common::Phase {
                name: common::PhaseName::Seer,
                data: HashMap::new(),
            };
            if living_players_with_role(common::PlayerRole::Seer, game_state.players.clone()) > 0 {
                game_state.phase = common::Phase {
                    name: common::PhaseName::Seer,
                    data: HashMap::new(),
                };
            }
            else if living_players_with_role(common::PlayerRole::Bodyguard, game_state.players.clone()) > 0 {
                game_state.phase = common::Phase {
                    name: common::PhaseName::Bodyguard,
                    data: HashMap::new(),
                };
            }
            else {
                game_state.phase = common::Phase {
                    name: common::PhaseName::Werewolf,
                    data: HashMap::new(),
                };
            }
        },
    }
    update_state(game_state, table_name)
}
