use std::env;
use std::collections::HashMap;

use rand::Rng;

use crate::ActionError;
use crate::helpers::{get_state, update_state};

#[derive(Deserialize, Serialize, Clone)]
struct StartEvent {
    action: String,
    data: EventData,
}

#[derive(Deserialize, Serialize, Clone)]
struct EventData {
    werewolves: u32,
    bodyguard: Option<bool>,
    seer: Option<bool>,
    lycan: Option<bool>,
    tanner: Option<bool>,
    code: String,
}

pub fn handle_start(e: common::ApiGatewayWebsocketProxyRequest) -> Result<(), ActionError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let event: StartEvent = serde_json::from_str(&body).unwrap();
    let table_name = env::var("tableName").unwrap();

    let current_game = get_state(table_name, event.data.code.clone());
    if let Ok(item) = current_game { 
        let data = event.data;
        move_to_day(e, item, data.werewolves, data.bodyguard.unwrap_or(false), data.seer.unwrap_or(true),
            data.lycan.unwrap_or(false), data.tanner.unwrap_or(false))
    } else {
        Err(ActionError::new(&"Game not found".to_string()))
    }
}

fn move_to_day(event: common::ApiGatewayWebsocketProxyRequest, mut game_state: common::GameState, werewolves: u32, bodyguard: bool, seer: bool,
        lycan: bool, tanner: bool) -> Result<(), ActionError> {
    let table_name = env::var("tableName").unwrap();

    let mut roles_count = werewolves + 1;
    if bodyguard { roles_count += 1 }
    if seer { roles_count += 1 }
    if lycan { roles_count += 1 }
    if tanner { roles_count += 1 }
    if roles_count > game_state.players.len() as u32 {
        error!("Roles: {}, Players: {}", roles_count, game_state.players.len());
        return Err(ActionError::new(&"More roles than players!".to_string()));
    }
    
    let mut roles: Vec<common::PlayerAttributes> = vec![];
    let num_villagers = game_state.players.len() as u32 - roles_count;
    if seer {
        roles.push(common::PlayerAttributes {
            role: common::PlayerRole::Seer,
            team: common::PlayerTeam::Good,
            alive: true,
            visible_to: vec![format!("{:?}", common::PlayerRole::Mod)],
        });
    }
    if bodyguard {
        roles.push(common::PlayerAttributes {
            role: common::PlayerRole::Bodyguard,
            team: common::PlayerTeam::Good,
            alive: true,
            visible_to: vec![format!("{:?}", common::PlayerRole::Mod)],
        });
    }
    if lycan {
        roles.push(common::PlayerAttributes {
            role: common::PlayerRole::Lycan,
            team: common::PlayerTeam::Good,
            alive: true,
            visible_to: vec![format!("{:?}", common::PlayerRole::Mod)],
        });
    }
    if tanner {
        roles.push(common::PlayerAttributes {
            role: common::PlayerRole::Tanner,
            team: common::PlayerTeam::Good,
            alive: true,
            visible_to: vec![format!("{:?}", common::PlayerRole::Mod)],
        });
    }
    for _ in 0..werewolves {
        roles.push(common::PlayerAttributes {
            role: common::PlayerRole::Werewolf,
            team: common::PlayerTeam::Evil,
            alive: true,
            visible_to: vec![format!("{:?}", common::PlayerRole::Mod), format!("{:?}", common::PlayerRole::Werewolf)],
        });
    }
    for _ in 0..num_villagers {
        roles.push(common::PlayerAttributes {
            role: common::PlayerRole::Villager,
            team: common::PlayerTeam::Good,
            alive: true,
            visible_to: vec![format!("{:?}", common::PlayerRole::Mod)],
        });
    }


    let new_players = create_new_players(game_state.clone(), roles, event.request_context.connection_id.unwrap());

    game_state.players = new_players;
    game_state.phase = common::Phase {
        name: common::PhaseName::Day,
        data: HashMap::new(),
    };

    update_state(game_state, table_name)
}

fn create_new_players(game_state: common::GameState, mut roles: Vec<common::PlayerAttributes>, connection_id: String) -> Vec<common::Player> {
    let mut new_players = vec![];
    let mut rng = rand::thread_rng();
    for player in &game_state.players {
        let mut new_player = player.clone();
        if player.id == connection_id.clone() {
            new_player.attributes = common::PlayerAttributes {
                role: common::PlayerRole::Mod,
                team: common::PlayerTeam::Unknown,
                alive: true,
                visible_to: vec!["All".to_string()],
            };
        }
        else {
            let role = roles.remove(rng.gen_range(0, roles.len()));
            new_player.attributes = role;
        }
        new_players.push(new_player);
    }
    new_players
}
