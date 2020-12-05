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
    player: String,
}

pub fn handle_seer(e: common::ApiGatewayWebsocketProxyRequest) -> Result<(), ActionError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let event: SleepEvent = serde_json::from_str(&body).unwrap();
    
    let table_name = env::var("tableName").unwrap();

    let current_game = get_state(table_name, e.clone(), event.data.code.clone());
    if let Some(item) = current_game { 
        move_to_werewolf(e, item, event.data.player) 
    } else {
        Err(ActionError::new(&"Game not found".to_string()))
    }
}

fn move_to_werewolf(event: common::ApiGatewayWebsocketProxyRequest, item: HashMap<String, AttributeValue>, see_player_name: String)
        -> Result<(), ActionError> {
    let table_name = env::var("tableName").unwrap();

    let mut game_state: common::GameState = serde_json::from_str(&item["data"].s.clone().unwrap()).unwrap();

    let players: Vec<common::Player> = game_state.players.clone().into_iter().filter(|p| p.id == event.request_context.connection_id.clone().unwrap()).collect();
    if players.len() != 1 {
        return Err(ActionError::new(&format!("Could not find player with connection ID: {:?}", event.request_context.connection_id.unwrap())));
    }
    else if game_state.phase.name != common::PhaseName::Seer {
        return Err(ActionError::new(&"Not a valid transition!".to_string()));
    }
    else if players[0].attributes.role != common::PlayerRole::Seer {
        return Err(ActionError::new(&"You are not the seer!".to_string()));
    }
    let see_player: Vec<common::Player> = game_state.players.clone().into_iter()
        .filter(|p| p.name == see_player_name).collect();
    if see_player.len() != 1 {
        return Err(ActionError::new(&"Invalid player to see!".to_string()));
    }
    else if see_player[0].attributes.visible_to.contains(&format!("{:?}", common::PlayerRole::Seer)) || !see_player[0].attributes.alive {
        return Err(ActionError::new(&"Player is already seen!".to_string()));
    }
    let mut new_players = game_state.players.clone();
    new_players.retain(|p| p.name != see_player_name);
    let mut new_attributes = see_player[0].attributes.clone();
    new_attributes.visible_to.push(format!("{:?}", common::PlayerRole::Seer));
    let mut new_seen_player = see_player[0].clone();
    new_seen_player.attributes = see_player[0].attributes.clone();
    new_seen_player.attributes = new_attributes;
    new_players.push(new_seen_player);
    game_state.players = new_players.clone();
    if living_players_with_role(common::PlayerRole::Bodyguard, new_players) > 0 {
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
