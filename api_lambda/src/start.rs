use lambda::error::HandlerError;

use std::env;
use std::collections::HashMap;

use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;

use dynomite::{
    dynamodb::{
        DynamoDb, AttributeValue, GetItemInput, GetItemOutput,
    },
};
use futures::Future;
use rand::Rng;

use crate::helpers::{send_error, endpoint, update_state, DDB, RT, RequestResult, RequestError};

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

pub fn handle_start(e: common::ApiGatewayWebsocketProxyRequest) -> Result<ApiGatewayProxyResponse, HandlerError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let event: StartEvent = serde_json::from_str(&body).unwrap();
    
    let table_name = env::var("tableName").unwrap();

    let mut ddb_keys = HashMap::new();
    ddb_keys.insert("lobby_id".to_string(), AttributeValue {
        s: Some(event.data.code.clone()),
        ..Default::default()
    });

    let result = DDB.with(|ddb| {
        ddb.get_item(GetItemInput {
            table_name: table_name.clone(),
            key: ddb_keys,
            ..GetItemInput::default()
        })
        .map(RequestResult::Get)
        .map_err(RequestError::Get)
    });

    match RT.with(|rt| rt.borrow_mut().block_on(result)) {
        Err(err) => {
            log::error!("failed to perform new game connection operation: {:?}", err);
            send_error(format!("Lobby not found: {:?}", err),
                e.request_context.connection_id.clone().unwrap(), endpoint(&e.request_context));
        },
        Ok(result) => {
            match result {
                RequestResult::Get(result) => {
                    let result: GetItemOutput = result;
                    match result.item {
                        None => {
                            error!("Lobby not found: {:?}", event.data.code);
                            send_error("Unable to find lobby".to_string(),
                                e.request_context.connection_id.clone().unwrap(), endpoint(&e.request_context));
                        },
                        Some(item) => {
                            let data = event.data;
                            move_to_day(e, item, data.werewolves, data.bodyguard.unwrap_or(false), data.seer.unwrap_or(true),
                                data.lycan.unwrap_or(false), data.tanner.unwrap_or(false));
                        },
                    }
                }
            }
        }
    }

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: HashMap::new(),
        multi_value_headers: HashMap::new(),
        body: None,
        is_base64_encoded: None,
    })
}

fn move_to_day(event: common::ApiGatewayWebsocketProxyRequest, item: HashMap<String, AttributeValue>, werewolves: u32, bodyguard: bool, seer: bool,
        lycan: bool, tanner: bool) {
    let table_name = env::var("tableName").unwrap();

    let mut game_state: common::GameState = serde_json::from_str(&item["data"].s.clone().unwrap()).unwrap();

    let mut roles_count = werewolves + 1;
    if bodyguard { roles_count += 1 }
    if seer { roles_count += 1 }
    if lycan { roles_count += 1 }
    if tanner { roles_count += 1 }
    if roles_count > game_state.players.len() as u32 {
        error!("Roles: {}, Players: {}", roles_count, game_state.players.len());
        send_error("More roles than players!".to_string(),
            event.request_context.connection_id.clone().unwrap(), endpoint(&event.request_context));
        return;
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


    let mut new_players = vec![];
    let mut rng = rand::thread_rng();
    for player in &game_state.players {
        let mut new_player = player.clone();
        if player.id == event.request_context.connection_id.clone().unwrap() {
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

    game_state.players = new_players;
    game_state.phase = common::Phase {
        name: common::PhaseName::Day,
        data: HashMap::new(),
    };

    update_state(item, game_state, table_name, event);
}
