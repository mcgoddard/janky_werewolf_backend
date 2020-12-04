use lambda::error::HandlerError;

use std::env;
use std::collections::HashMap;

use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;

use dynomite::{
    dynamodb::{
        AttributeValue,
    },
};

use crate::helpers::{get_state, send_error, endpoint, update_state, living_players_with_role};

#[derive(Deserialize, Serialize, Clone)]
struct SleepEvent {
    action: String,
    data: EventData,
}

#[derive(Deserialize, Serialize, Clone)]
struct EventData {
    code: String,
}

pub fn handle_sleep(e: common::ApiGatewayWebsocketProxyRequest) -> Result<ApiGatewayProxyResponse, HandlerError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let event: SleepEvent = serde_json::from_str(&body).unwrap();
    
    let table_name = env::var("tableName").unwrap();

    let current_game = get_state(table_name, e.clone(), event.data.code);
    if let Some(item) = current_game { move_to_sleep(e, item) }

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: HashMap::new(),
        multi_value_headers: HashMap::new(),
        body: None,
        is_base64_encoded: None,
    })
}

fn move_to_sleep(event: common::ApiGatewayWebsocketProxyRequest, item: HashMap<String, AttributeValue>) {
    let table_name = env::var("tableName").unwrap();

    let mut game_state: common::GameState = serde_json::from_str(&item["data"].s.clone().unwrap()).unwrap();

    let players: Vec<common::Player> = game_state.players.clone().into_iter().filter(|p| p.id == event.request_context.connection_id.clone().unwrap()).collect();
    if players.len() != 1 {
        send_error(format!("Could not find player with connection ID: {:?}", event.request_context.connection_id.clone().unwrap()),
                event.request_context.connection_id.clone().unwrap(), endpoint(&event.request_context));
    }
    else if game_state.phase.name != common::PhaseName::Day {
        send_error("Not a valid transition!".to_string(),
            event.request_context.connection_id.clone().unwrap(), endpoint(&event.request_context));
    }
    else if players[0].attributes.role != common::PlayerRole::Mod {
        send_error("You are not the moderator!".to_string(),
            event.request_context.connection_id.clone().unwrap(), endpoint(&event.request_context));
    }
    else {
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
        update_state(item, game_state, table_name, event);
    }
}
