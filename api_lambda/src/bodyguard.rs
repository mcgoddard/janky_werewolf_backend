use lambda::error::HandlerError;

use std::env;
use std::collections::HashMap;

use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;

use dynomite::{
    dynamodb::{
        AttributeValue,
    },
};

use crate::helpers::{get_state, send_error, endpoint, update_state};

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

pub fn handle_bodyguard(e: common::ApiGatewayWebsocketProxyRequest) -> Result<ApiGatewayProxyResponse, HandlerError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let event: BodyguardEvent = serde_json::from_str(&body).unwrap();
    
    let table_name = env::var("tableName").unwrap();

    let current_game = get_state(table_name, e.clone(), event.data.code.clone());
    if let Some(item) = current_game { move_to_werewolf(e, item, event.data.player) }

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: HashMap::new(),
        multi_value_headers: HashMap::new(),
        body: None,
        is_base64_encoded: None,
    })
}

fn move_to_werewolf(event: common::ApiGatewayWebsocketProxyRequest, item: HashMap<String, AttributeValue>, protect_player_name: String) {
    let table_name = env::var("tableName").unwrap();

    let mut game_state: common::GameState = serde_json::from_str(&item["data"].s.clone().unwrap()).unwrap();

    let players: Vec<common::Player> = game_state.players.clone().into_iter().filter(|p| p.id == event.request_context.connection_id.clone().unwrap()).collect();
    if players.len() != 1 {
        send_error(format!("Could not find player with connection ID: {:?}", event.request_context.connection_id.clone().unwrap()),
                event.request_context.connection_id.clone().unwrap(), endpoint(&event.request_context));
    }
    else if game_state.phase.name != common::PhaseName::Bodyguard {
        send_error("Not a valid transition!".to_string(),
            event.request_context.connection_id.clone().unwrap(), endpoint(&event.request_context));
    }
    else if players[0].attributes.role != common::PlayerRole::Bodyguard {
        send_error("You are not the bodyguard!".to_string(),
            event.request_context.connection_id.clone().unwrap(), endpoint(&event.request_context));
    }
    else {
        let protect_player: Vec<common::Player> = game_state.players.clone().into_iter()
            .filter(|p| p.name == protect_player_name && p.attributes.alive).collect();
        if protect_player.len() != 1 || protect_player_name == players[0].name || 
            game_state.internal_state.get("last_guarded").unwrap_or(&"".to_string()).clone() == protect_player_name {
            send_error("Invalid player to protect!".to_string(),
                event.request_context.connection_id.clone().unwrap(), endpoint(&event.request_context));
        }
        else {
            let mut internal_state = HashMap::new();
            internal_state.insert("last_guarded".to_string(), protect_player_name);
            game_state.internal_state = internal_state;
            game_state.phase = common::Phase {
                name: common::PhaseName::Werewolf,
                data: HashMap::new(),
            };
            update_state(item, game_state, table_name, event);
        }
    }
}
