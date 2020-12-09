extern crate lambda;
extern crate serde_derive;
extern crate simple_logger;

use futures::future::join_all;
use futures::executor::block_on;
use bytes::Bytes;

use std::env;
use std::collections::HashMap;
use std::time::Instant;

use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;
use rusoto_apigatewaymanagementapi::{
    ApiGatewayManagementApi, ApiGatewayManagementApiClient, PostToConnectionRequest, PostToConnectionError,
};
use rusoto_core::{Region, RusotoError};
use serde_json::{json, Value};
use lambda::{lambda, Context};

use common::GameState;

type LambdaError = Box<dyn std::error::Error + Send + Sync + 'static>;

thread_local!(
    pub static API_GW: ApiGatewayManagementApiClient = ApiGatewayManagementApiClient::new(Region::Custom {
        name: Region::EuWest2.name().into(),
        endpoint: endpoint(),
    });
);

#[lambda]
#[tokio::main]
async fn main(e: common::DDBStreamEvent, _c: Context) -> Result<ApiGatewayProxyResponse, LambdaError> {
    let start = Instant::now();
    match e.records {
        Some(records) => {
            for record in &records {
                process_record(record).await;
            }
        },
        None => log::warn!("No records in event, empty execution..."),
    }
    let duration = start.elapsed();
    println!("Time elapsed in processing is: {:?}", duration);


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
                                    let game_state: GameState = serde_dynamodb::from_hashmap(new_image.clone()).unwrap();
                                    let players = game_state.players.clone();
                                    let broadcasts = players.into_iter().map(|p| {
                                        let filtered_state = filter_state(&p, game_state.clone());
                                        let value_state = serde_json::to_value(&filtered_state).unwrap();
                                        let mut map_state: HashMap<String, Value> = serde_json::from_value(value_state).unwrap();
                                        let lobby_id = map_state.remove("lobby_id").unwrap();
                                        map_state.insert("lobbyId".to_string(), lobby_id);
                                        map_state.remove("ttl");
                                        broadcast(p, map_state)
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

async fn broadcast(player: common::Player, game_state: HashMap<String, Value>) -> Result<(), RusotoError<PostToConnectionError>> {
    API_GW.with(|api_gw| {
        block_on(api_gw.post_to_connection(PostToConnectionRequest {
            connection_id: player.id.clone(),
            data: Bytes::from(json!({ "game_state": game_state }).to_string()),
        }))
    })
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
