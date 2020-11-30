#[macro_use]
extern crate lambda_runtime as lambda;
extern crate serde_derive;
extern crate simple_logger;

use lambda::error::HandlerError;

use std::error::Error;
use std::env;
use std::collections::HashMap;
use std::thread;

use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;
use rusoto_apigatewaymanagementapi::{
    ApiGatewayManagementApi, ApiGatewayManagementApiClient, PostToConnectionRequest,
};
use rusoto_core::Region;
use serde_json::json;

use common::types;

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::init_with_level(log::Level::Info)?;
    lambda!(my_handler);

    Ok(())
}

fn my_handler(e: types::DDBStreamEvent, _c: lambda::Context) -> Result<ApiGatewayProxyResponse, HandlerError> {
    match e.records {
        Some(records) => {
            for record in &records {
                process_record(record);
            }
        },
        None => log::warn!("No records in event, empty execution..."),
    }

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: HashMap::new(),
        multi_value_headers: HashMap::new(),
        body: None,
        is_base64_encoded: None,
    })
}

fn process_record(record: &types::DDBRecord) {
    match &record.dynamodb {
        Some(stream_record) => {
            match &stream_record.stream_view_type {
                Some(s) => {
                    match s.as_str() {
                        "NEW_IMAGE" => {
                            match &stream_record.new_image {
                                Some(new_image) => {
                                    let new_image: types::GameState = serde_json::from_str(&new_image.data["S"]).unwrap();
                                    let players = new_image.players.clone();
                                    let broadcasts = players.into_iter().map(|p| {
                                        let new_image = new_image.clone();
                                        thread::spawn(move || {
                                            let filtered_state = filter_state(&p, new_image);
                                            broadcast(&p, filtered_state);
                                        })
                                    }).collect::<Vec<_>>();
                                    for b in broadcasts {
                                        if let Err(err) = b.join() {
                                            log::error!("Error broadcasting: {:?}", err);
                                        }
                                    }
                                },
                                None => log::error!("No new image"),
                            }
                        },
                        s => log::error!("unable to process stream view: {:?}", s),
                    }
                },
                None => log::error!("unable to process stream view, no type"),
            }
        },
        None => log::warn!("No stream record"),
    }
}

fn broadcast(player: &types::Player, game_state: types::GameState) {
    let client = ApiGatewayManagementApiClient::new(Region::Custom {
        name: Region::EuWest2.name().into(),
        endpoint: endpoint(),
    });
    let result = client.post_to_connection(PostToConnectionRequest {
                    connection_id: player.id.clone(),
                    data: serde_json::to_vec(&json!({ "game_state": game_state })).unwrap_or_default(),
                }).sync();
    if let Err(e) = result { log::error!("Unable to send state: {:?}", e) }
}

fn filter_state(player: &types::Player, game_state: types::GameState) -> types::GameState {
    let mut new_state = game_state.clone();
    if game_state.phase.name == types::PhaseName::Werewolf && 
        !vec![types::PlayerRole::Mod, types::PlayerRole::Werewolf].contains(&player.attributes.role) {
            new_state.phase.data = HashMap::new();
    }
    if game_state.phase.name == types::PhaseName::Bodyguard && player.attributes.role == types::PlayerRole::Bodyguard {
        let mut phase_data = HashMap::new();
        phase_data.insert("last_guarded".to_string(), game_state.internal_state.get("last_guarded").unwrap_or(&"".to_string()).clone());
        new_state.phase.data = phase_data;
    }
    new_state.internal_state = HashMap::new();
    new_state.players = new_state.players.into_iter().map(|p| {
        let mut new_attributes = p.attributes.clone();
        let mut new_player = p.clone();
        new_player.secret = "".to_string();
        new_player.id = "".to_string();
        if game_state.phase.name != types::PhaseName::End {
            if p.name != player.name && new_attributes.alive && new_attributes.role != types::PlayerRole::Mod {
                if !new_attributes.visible_to.contains(&format!("{:?}", player.attributes.role)) {
                    new_attributes.role = types::PlayerRole::Unknown;
                    new_attributes.team = types::PlayerTeam::Unknown;
                }
                else if player.attributes.role == types::PlayerRole::Seer {
                    new_attributes.role = types::PlayerRole::Unknown;
                    if p.attributes.role == types::PlayerRole::Lycan {
                        new_attributes.team = types::PlayerTeam::Evil;
                    }
                }
            }
            new_attributes.visible_to = vec![];
            new_player.attributes = new_attributes;
        }
        new_player
    }).collect();
    new_state
}

fn endpoint() -> String {
    let domain_name = env::var("domainName").unwrap();
    let stage = env::var("stage").unwrap();
    format!("https://{}/{}", domain_name, stage)
}
