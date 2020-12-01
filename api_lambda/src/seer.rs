use lambda::error::HandlerError;

use std::env;
use std::collections::HashMap;

use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;

use dynomite::{
    dynamodb::{
        AttributeValue,
    },
};

use common::{types, helpers};

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

pub fn handle_seer(e: types::ApiGatewayWebsocketProxyRequest) -> Result<ApiGatewayProxyResponse, HandlerError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let event: SleepEvent = serde_json::from_str(&body).unwrap();
    
    let table_name = env::var("tableName").unwrap();

    let current_game = helpers::get_state(table_name, e.clone(), event.data.code.clone());
    if let Some(item) = current_game { move_to_werewolf(e, item, event.data.player) }

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: HashMap::new(),
        multi_value_headers: HashMap::new(),
        body: None,
        is_base64_encoded: None,
    })
}

fn move_to_werewolf(event: types::ApiGatewayWebsocketProxyRequest, item: HashMap<String, AttributeValue>, see_player_name: String) {
    let table_name = env::var("tableName").unwrap();

    let mut game_state: types::GameState = serde_json::from_str(&item["data"].s.clone().unwrap()).unwrap();

    let players: Vec<types::Player> = game_state.players.clone().into_iter().filter(|p| p.id == event.request_context.connection_id.clone().unwrap()).collect();
    if players.len() != 1 {
        helpers::send_error(format!("Could not find player with connection ID: {:?}", event.request_context.connection_id.clone().unwrap()),
                event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
    }
    else if game_state.phase.name != types::PhaseName::Seer {
        helpers::send_error("Not a valid transition!".to_string(),
            event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
    }
    else if players[0].attributes.role != types::PlayerRole::Seer {
        helpers::send_error("You are not the seer!".to_string(),
            event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
    }
    else {
        let see_player: Vec<types::Player> = game_state.players.clone().into_iter()
            .filter(|p| p.name == see_player_name).collect();
        if see_player.len() != 1 {
            helpers::send_error("Invalid player to see!".to_string(),
                event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
        }
        else if see_player[0].attributes.visible_to.contains(&format!("{:?}", types::PlayerRole::Seer)) || !see_player[0].attributes.alive {
            helpers::send_error("Player is already seen!".to_string(),
                event.request_context.connection_id.clone().unwrap(), helpers::endpoint(&event.request_context));
        }
        else {
            let mut new_players = game_state.players.clone();
            new_players.retain(|p| p.name != see_player_name);
            let mut new_attributes = see_player[0].attributes.clone();
            new_attributes.visible_to.push(format!("{:?}", types::PlayerRole::Seer));
            let mut new_seen_player = see_player[0].clone();
            new_seen_player.attributes = see_player[0].attributes.clone();
            new_seen_player.attributes = new_attributes;
            new_players.push(new_seen_player);
            game_state.players = new_players.clone();
            if helpers::living_players_with_role(types::PlayerRole::Bodyguard, new_players) > 0 {
                game_state.phase = types::Phase {
                    name: types::PhaseName::Bodyguard,
                    data: HashMap::new(),
                };
            }
            else {
                game_state.phase = types::Phase {
                    name: types::PhaseName::Werewolf,
                    data: HashMap::new(),
                };
            }
            helpers::update_state(item, game_state, table_name, event);
        }
    }
}
