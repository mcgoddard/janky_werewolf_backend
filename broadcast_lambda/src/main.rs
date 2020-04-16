#[macro_use]
extern crate lambda_runtime as lambda;
extern crate serde_derive;
extern crate simple_logger;

use lambda::error::HandlerError;

use std;
use std::error::Error;
use std::env;
use std::collections::HashMap;

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
                                    for player in &players {
                                        let filtered_state = filter_state(player, new_image.clone());
                                        broadcast(player, filtered_state);
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
    let result = client.clone().post_to_connection(PostToConnectionRequest {
                    connection_id: player.id.clone(),
                    data: serde_json::to_vec(&json!({ "game_state": game_state.clone() })).unwrap_or_default(),
                }).sync();
    match result {
        Err(e) => log::error!("Unable to send state: {:?}", e),
        _ => (),
    }
}

fn filter_state(player: &types::Player, game_state: types::GameState) -> types::GameState {
    let mut new_state = game_state.clone();
    if game_state.phase.name == types::PhaseName::Werewolf && 
        !vec![types::PlayerRole::Mod, types::PlayerRole::Werewolf].contains(&player.attributes.as_ref().unwrap().role) {
            new_state.phase.data = HashMap::new();
    }
    new_state.players = new_state.players.into_iter().map(|p| {
        let new_attributes_option = p.attributes.clone();
        let mut new_player = p.clone();
        new_player.secret = "".to_string();
        new_player.id = "".to_string();
        if game_state.phase.name != types::PhaseName::End {
            if let Some(mut new_attributes) = new_attributes_option {
                if let Some(player_attributes) = player.attributes.clone() {
                    if p.name != player.name && new_attributes.alive && new_attributes.role != types::PlayerRole::Mod {
                        if !new_attributes.visible_to.contains(&format!("{:?}", player_attributes.role.clone())) {
                            new_attributes.role = types::PlayerRole::Unknown;
                            new_attributes.team = types::PlayerTeam::Unknown;
                        }
                        else if player.attributes.as_ref().unwrap().role == types::PlayerRole::Seer {
                            new_attributes.role = types::PlayerRole::Unknown;
                        }
                    }
                    new_attributes.visible_to = vec![];
                    new_player.attributes = Some(new_attributes);
                }
            }
        }
        return new_player;
    }).collect();
    return new_state;
}

fn endpoint() -> String {
    let domain_name = env::var("domainName").unwrap();
    let stage = env::var("stage").unwrap();
    format!("https://{}/{}", domain_name, stage)
}
