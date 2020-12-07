use std::env;
use std::collections::HashMap;

use crate::ActionError;
use crate::helpers::{get_state, update_state};

#[derive(Deserialize, Serialize, Clone)]
struct BodyguardEvent {
    action: String,
    data: EventData,
}

#[derive(Deserialize, Serialize, Clone)]
struct EventData {
    code: String,
    player: String,
}

pub async fn handle_bodyguard(e: common::ApiGatewayWebsocketProxyRequest) -> Result<(), ActionError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let event: BodyguardEvent = serde_json::from_str(&body).unwrap();
    
    let table_name = env::var("tableName").unwrap();

    let current_game = get_state(table_name, event.data.code.clone()).await;
    if let Ok(item) = current_game {
        move_to_werewolf(e, item, event.data.player).await
    } else {
        Err(ActionError::new(&"Game not found".to_string()))
    }
}

async fn move_to_werewolf(event: common::ApiGatewayWebsocketProxyRequest, mut game_state: common::GameState, protect_player_name: String) 
        -> Result<(), ActionError> {
    let table_name = env::var("tableName").unwrap();

    let players: Vec<common::Player> = game_state.players.clone().into_iter().filter(|p| p.id == event.request_context.connection_id.clone().unwrap()).collect();
    if players.len() != 1 {
        return Err(ActionError::new(&format!("Could not find player with connection ID: {:?}",
            event.request_context.connection_id.unwrap())));
    }
    else if game_state.phase.name != common::PhaseName::Bodyguard {
        return Err(ActionError::new(&"Not a valid transition!".to_string()));
    }
    else if players[0].attributes.role != common::PlayerRole::Bodyguard {
        return Err(ActionError::new(&"You are not the bodyguard!".to_string()));
    }
    let protect_player: Vec<common::Player> = game_state.players.clone().into_iter()
        .filter(|p| p.name == protect_player_name && p.attributes.alive).collect();
    if protect_player.len() != 1 || protect_player_name == players[0].name || 
        game_state.internal_state.get("last_guarded").unwrap_or(&"".to_string()).clone() == protect_player_name {
        return Err(ActionError::new(&"Invalid player to protect!".to_string()));
    }
    let mut internal_state = HashMap::new();
    internal_state.insert("last_guarded".to_string(), protect_player_name);
    game_state.internal_state = internal_state;
    game_state.phase = common::Phase {
        name: common::PhaseName::Werewolf,
        data: HashMap::new(),
    };
    update_state(game_state, table_name).await
}
