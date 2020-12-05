use std::env;
use std::collections::HashMap;

use dynomite::{
    dynamodb::{
        AttributeValue,
    },
};

use crate::ActionError;
use crate::helpers::{get_state, update_state, living_players_with_role};

#[derive(Deserialize, Serialize, Clone)]
struct SleepEvent {
    action: String,
    data: EventData,
}

#[derive(Deserialize, Serialize, Clone)]
struct EventData {
    code: String,
}

pub fn handle_sleep(e: common::ApiGatewayWebsocketProxyRequest) -> Result<(), ActionError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let event: SleepEvent = serde_json::from_str(&body).unwrap();
    
    let table_name = env::var("tableName").unwrap();

    let current_game = get_state(table_name, e.clone(), event.data.code);
    if let Some(item) = current_game { 
        move_to_sleep(e, item)
    } else {
        Err(ActionError::new(&"Game not found".to_string()))
    }
}

fn move_to_sleep(event: common::ApiGatewayWebsocketProxyRequest, item: HashMap<String, AttributeValue>) 
        -> Result<(), ActionError> {
    let table_name = env::var("tableName").unwrap();

    let mut game_state: common::GameState = serde_json::from_str(&item["data"].s.clone().unwrap()).unwrap();

    let players: Vec<common::Player> = game_state.players.clone().into_iter().filter(|p| p.id == event.request_context.connection_id.clone().unwrap()).collect();
    if players.len() != 1 {
        return Err(ActionError::new(&format!("Could not find player with connection ID: {:?}", 
            event.request_context.connection_id.unwrap())));
    }
    else if game_state.phase.name != common::PhaseName::Day {
        return Err(ActionError::new(&"Not a valid transition!".to_string()));
    }
    else if players[0].attributes.role != common::PlayerRole::Mod {
        return Err(ActionError::new(&"You are not the moderator!".to_string()));
    }
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
    update_state(item, game_state, table_name)
}
