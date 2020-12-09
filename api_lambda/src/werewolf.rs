use std::env;
use std::collections::HashMap;

use crate::ActionError;
use crate::helpers::{get_state, update_state, living_players_with_role, check_game_over};

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

pub fn handle_werewolf(e: common::ApiGatewayWebsocketProxyRequest) -> Result<(), ActionError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let event: WerewolfEvent = serde_json::from_str(&body).unwrap();
    
    let table_name = env::var("tableName").unwrap();

    let current_game = get_state(table_name, event.data.code.clone());
    if let Ok(item) = current_game {
        werewolf(e, item, event.data.player)
    } else {
        Err(ActionError::new(&"Game not found".to_string()))
    }
}

fn werewolf(event: common::ApiGatewayWebsocketProxyRequest, mut game_state: common::GameState, eat_player_name: String)
        -> Result<(), ActionError> {
    let table_name = env::var("tableName").unwrap();

    let players: Vec<common::Player> = game_state.players.clone().into_iter().filter(|p| p.id == event.request_context.connection_id.clone().unwrap()).collect();
    if players.len() != 1 {
        return Err(ActionError::new(&format!("Could not find player with connection ID: {:?}",
            event.request_context.connection_id.unwrap())));
    }
    else if game_state.phase.name != common::PhaseName::Werewolf {
        return Err(ActionError::new(&"Not a valid transition!".to_string()));
    }
    else if players[0].attributes.role != common::PlayerRole::Werewolf {
        return Err(ActionError::new(&"You are not a werewolf!".to_string()));
    }
    let eat_player: Vec<common::Player> = game_state.players.clone().into_iter()
        .filter(|p| p.name == eat_player_name).collect();
    if eat_player.len() != 1 || !eat_player[0].attributes.alive || eat_player[0].attributes.team != common::PlayerTeam::Good {
        return Err(ActionError::new(&"Invalid player to eat!".to_string()));
    }
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
    update_state(game_state, table_name)
}
