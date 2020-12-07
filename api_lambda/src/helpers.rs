use std::collections::HashMap;

use rusoto_apigatewaymanagementapi::{
    ApiGatewayManagementApi, ApiGatewayManagementApiClient, PostToConnectionRequest,
};
use rusoto_core::Region;
use serde_json::json;
use dynomite::{
    dynamodb::{
        DynamoDb, DynamoDbClient, PutItemInput, AttributeValue, GetItemInput,
    }, FromAttributes
};

use crate::ActionError;

pub fn send_error(message: String, connection_id: String, endpoint: String) {
    let client = ApiGatewayManagementApiClient::new(Region::Custom {
        name: Region::EuWest2.name().into(),
        endpoint,
    });
    let result = client.post_to_connection(PostToConnectionRequest {
                    connection_id,
                    data: serde_json::to_vec(&json!({ "message": message })).unwrap_or_default(),
                }).sync();
    if let Err(e) = result { error!("Error sending error: {:?}", e) }
}

pub fn endpoint(ctx: &common::ApiGatewayWebsocketProxyRequestContext) -> String {
    match &ctx.domain_name {
        Some(domain) => (
            match &ctx.stage {
                Some(stage) => (
                    format!("https://{}/{}", domain, stage)
                ),
                None => panic!("No stage on request context"),
            }
        ),
        None => panic!("No domain on request context"),
    }
}

pub async fn update_state(mut game_state: common::GameState, table_name: String) -> Result<(), ActionError> {
    game_state.version += 1;
    let condition_expression = "version < :version".to_string();
    let mut attribute_values = HashMap::default();
    attribute_values.insert(":version".to_string(), AttributeValue {
        n: Some(game_state.version.to_string()),
        ..Default::default()
    });
    let ddb = DynamoDbClient::new(Default::default());
    let result = ddb.put_item(PutItemInput {
            table_name,
            condition_expression: Some(condition_expression),
            item: game_state.into(),
            expression_attribute_values: Some(attribute_values),
            ..PutItemInput::default()
        })
        .await;

    match result {
        Ok(_) => Ok(()),
        Err(err) => {
            error!("Error saving state, please try again: {:?}", err);
            Err(ActionError::new(&"Error saving state, please try again".to_string()))
        },
    }
}

pub async fn get_state(table_name: String, lobby_id: String) -> Result<common::GameState, ActionError> {
    let mut ddb_keys = HashMap::new();
    ddb_keys.insert("lobby_id".to_string(), AttributeValue {
        s: Some(lobby_id.clone()),
        ..Default::default()
    });

    let ddb = DynamoDbClient::new(Default::default());
    let item = ddb.get_item(GetItemInput {
        table_name: table_name.clone(),
        key: ddb_keys,
        ..GetItemInput::default()
    }).await;

    match item {
        Ok(i) => {
            match i.item.map(common::GameState::from_attrs) {
                Some(i) => {
                    match i {
                        Ok(gs) => Ok(gs),
                        Err(e) => {
                            error!("Game state corrupted: {}", e);
                            Err(ActionError::new(&"Game state corrupted".to_string()))
                        },
                    }
                },
                None => Err(ActionError::new(&"Lobby not found".to_string())),
            }
        },
        Err(e) => {
            error!("Error fetching lobby: {:?}", e);
            Err(ActionError::new(&"Error fetching lobby".to_string()))
        },
    }
}

pub fn check_game_over(players: Vec<common::Player>) -> Option<Vec<common::PlayerTeam>> {
    let good_players: Vec<common::Player> = players.clone().into_iter().filter(|p| p.attributes.team == common::PlayerTeam::Good && p.attributes.alive).collect();
    let evil_players: Vec<common::Player> = players.clone().into_iter().filter(|p| p.attributes.team == common::PlayerTeam::Evil && p.attributes.alive).collect();
    let mut winners = None;
    if evil_players.len() >= good_players.len() || evil_players.is_empty() {
        let mut teams = vec![];
        if players.clone().into_iter().filter(|p| p.attributes.role == common::PlayerRole::Tanner).count() > 0 && 
            living_players_with_role(common::PlayerRole::Tanner, players.clone()) < 1 {
            teams.push(common::PlayerTeam::Tanner);
        }
        match players.into_iter().filter(|p| p.attributes.team == common::PlayerTeam::Evil && p.attributes.alive).count() {
            0 => {
                teams.push(common::PlayerTeam::Good);
            },
            _ => {
                teams.push(common::PlayerTeam::Evil);
            }
        };
        winners = Some(teams);
    }
    winners
}

pub fn living_players_with_role(role: common::PlayerRole, players: Vec<common::Player>) -> u32 {
    players.into_iter().filter(|p| p.attributes.role == role && p.attributes.alive).count() as u32
}
