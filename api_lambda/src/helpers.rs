use std::collections::HashMap;
use std::cell::RefCell;

use rusoto_apigatewaymanagementapi::{
    ApiGatewayManagementApi, ApiGatewayManagementApiClient, PostToConnectionRequest,
};
use rusoto_core::Region;
use rusoto_core::RusotoError;
use serde_json::json;
use dynomite::{
    dynamodb::{
        DynamoDb, DynamoDbClient, PutItemInput, AttributeValue, GetItemInput, GetItemOutput, PutItemError, GetItemError
    },
};
use tokio::runtime::Runtime;
use futures::Future;

use crate::ActionError;

thread_local!(
    pub static DDB: DynamoDbClient = DynamoDbClient::new(Default::default());
);

thread_local!(
    pub static RT: RefCell<Runtime> =
        RefCell::new(Runtime::new().expect("failed to initialize runtime"));
);

#[derive(Debug)]
pub enum RequestResult {
    Get(GetItemOutput),
}

#[derive(Debug)]
pub enum RequestError {
    Connect(RusotoError<PutItemError>),
    Get(RusotoError<GetItemError>),
}

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

pub fn update_state(item: HashMap<String, AttributeValue, std::collections::hash_map::RandomState>, game_state: common::GameState,
                    table_name: String) -> Result<(), ActionError> {
    let mut new_item = item;
    new_item.insert("version".to_string(), AttributeValue {
        n: Some(format!("{}", new_item["version"].n.clone().unwrap().parse::<i32>().unwrap() + 1)),
        ..Default::default()
    });
    let d = json!(game_state);
    new_item.insert("data".to_string(), AttributeValue {
        s: Some(d.to_string()),
        ..Default::default()
    });
    let condition_expression = "version < :version".to_string();
    let mut attribute_values = HashMap::default();
    attribute_values.insert(":version".to_string(), AttributeValue {
        n: Some(new_item["version"].n.clone().unwrap()),
        ..Default::default()
    });
    let result = DDB.with(|ddb| {
        ddb.put_item(PutItemInput {
            table_name,
            condition_expression: Some(condition_expression),
            item: new_item,
            expression_attribute_values: Some(attribute_values),
            ..PutItemInput::default()
        })
        .map(drop)
        .map_err(RequestError::Connect)
    });

    if let Err(err) = RT.with(|rt| rt.borrow_mut().block_on(result)) {
        log::error!("Error saving state: {:?}", err);
        return Err(ActionError::new(&format!("Error saving state, please try again: {:?}", err)))
    };

    Ok(())
}

pub fn get_state(table_name: String, event: common::ApiGatewayWebsocketProxyRequest, 
             lobby_id: String) -> Option<HashMap<String, AttributeValue>> {
    let mut ddb_keys = HashMap::new();
    ddb_keys.insert("lobby_id".to_string(), AttributeValue {
        s: Some(lobby_id.clone()),
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
            log::error!("failed to find the game: {:?}", err);
            send_error(format!("Lobby not found: {:?}", err),
                event.request_context.connection_id.clone().unwrap(), endpoint(&event.request_context));
        },
        Ok(result) => {
            match result {
                RequestResult::Get(result) => {
                    let result: GetItemOutput = result;
                    match result.item {
                        None => {
                            error!("Lobby not found: {:?}", lobby_id);
                            send_error("Unable to find lobby".to_string(),
                                event.request_context.connection_id.clone().unwrap(), endpoint(&event.request_context));
                        },
                        Some(item) => {
                            return Some(item);
                        },
                    }
                }
            }
        }
    }
    None
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
