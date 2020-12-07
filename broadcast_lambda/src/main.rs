#[macro_use]
extern crate lambda_runtime as lambda;
extern crate serde_derive;
extern crate simple_logger;

use lambda::error::HandlerError;

use futures::executor::block_on;
use futures::future::join_all;
use bytes::Bytes;

use std::error::Error;
use std::env;
use std::collections::HashMap;

use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;
use rusoto_apigatewaymanagementapi::{
    ApiGatewayManagementApi, ApiGatewayManagementApiClient, PostToConnectionRequest, PostToConnectionError,
};
use rusoto_core::{Region, RusotoError};
use serde_json::json;

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::init_with_level(log::Level::Info)?;
    lambda!(my_handler);

    Ok(())
}

fn my_handler(e: common::DDBStreamEvent, _c: lambda::Context) -> Result<ApiGatewayProxyResponse, HandlerError> {
    match e.records {
        Some(records) => {
            for record in &records {
                block_on(process_record(record));
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

async fn process_record(record: &common::DDBRecord) {
    match &record.dynamodb {
        Some(stream_record) => {
            match &stream_record.stream_view_type {
                Some(s) => {
                    match s.as_str() {
                        "NEW_IMAGE" => {
                            match &stream_record.new_image {
                                Some(new_image) => {
                                    let players = new_image.players.clone();
                                    let broadcasts = players.into_iter().map(|p| {
                                        let filtered_state = filter_state(&p, new_image.clone());
                                        broadcast(p, filtered_state)
                                    }).collect::<Vec<_>>();
                                    let results = join_all(broadcasts).await;
                                    for r in results.into_iter() {
                                        if let Err(err) = r {
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

async fn broadcast(player: common::Player, game_state: common::GameState) -> Result<(), RusotoError<PostToConnectionError>> {
    let client = ApiGatewayManagementApiClient::new(Region::Custom {
        name: Region::EuWest2.name().into(),
        endpoint: endpoint(),
    });
    client.post_to_connection(PostToConnectionRequest {
        connection_id: player.id.clone(),
        data: Bytes::from(json!({ "game_state": game_state }).to_string()),
    }).await
}

fn filter_state(player: &common::Player, game_state: common::GameState) -> common::GameState {
    let mut new_state = game_state.clone();
    if game_state.phase.name == common::PhaseName::Werewolf && 
        !vec![common::PlayerRole::Mod, common::PlayerRole::Werewolf].contains(&player.attributes.role) {
            new_state.phase.data = HashMap::new();
    }
    if game_state.phase.name == common::PhaseName::Bodyguard && player.attributes.role == common::PlayerRole::Bodyguard {
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
        if game_state.phase.name != common::PhaseName::End {
            if p.name != player.name && new_attributes.alive && new_attributes.role != common::PlayerRole::Mod {
                if !new_attributes.visible_to.contains(&format!("{:?}", player.attributes.role)) {
                    new_attributes.role = common::PlayerRole::Unknown;
                    new_attributes.team = common::PlayerTeam::Unknown;
                }
                else if player.attributes.role == common::PlayerRole::Seer {
                    new_attributes.role = common::PlayerRole::Unknown;
                    if p.attributes.role == common::PlayerRole::Lycan {
                        new_attributes.team = common::PlayerTeam::Evil;
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
    let domain_name = env::var("apiUrl").unwrap();
    domain_name.replace("wss://", "https://")
}
