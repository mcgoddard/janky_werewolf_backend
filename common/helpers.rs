use std::collections::HashMap;
use std::cell::RefCell;

use rusoto_apigatewaymanagementapi::{
    ApiGatewayManagementApi, ApiGatewayManagementApiClient, PostToConnectionRequest,
};
use rusoto_core::Region;
use serde_json::json;
use dynomite::{
    dynamodb::{
        DynamoDb, DynamoDbClient, PutItemInput, AttributeValue, GetItemInput, GetItemOutput,
    },
};
use tokio::runtime::Runtime;
use futures::Future;

use super::types;

thread_local!(
    pub static DDB: DynamoDbClient = DynamoDbClient::new(Default::default());
);

thread_local!(
    pub static RT: RefCell<Runtime> =
        RefCell::new(Runtime::new().expect("failed to initialize runtime"));
);

pub fn send_error(message: String, connection_id: String, endpoint: String) {
    let client = ApiGatewayManagementApiClient::new(Region::Custom {
        name: Region::EuWest2.name().into(),
        endpoint: endpoint,
    });
    let result = client.clone().post_to_connection(PostToConnectionRequest {
                    connection_id: connection_id,
                    data: serde_json::to_vec(&json!({ "message": message })).unwrap_or_default(),
                }).sync();
    match result {
        Err(e) => error!("Error sending error: {:?}", e),
        _ => (),
    }
}

pub fn endpoint(ctx: &types::ApiGatewayWebsocketProxyRequestContext) -> String {
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

pub fn update_state(item: HashMap<String, AttributeValue>, game_state: types::GameState,
                table_name: String, event: types::ApiGatewayWebsocketProxyRequest) {
    let mut new_item = item.clone();
    new_item.insert("version".to_string(), AttributeValue {
        n: Some(format!("{}", new_item["version"].n.clone().unwrap().parse::<i32>().unwrap() + 1)),
        ..Default::default()
    });
    let d = json!(game_state);
    new_item.insert("data".to_string(), AttributeValue {
        s: Some(d.to_string()),
        ..Default::default()
    });
    let condition_expression = format!("version < :version");
    let mut attribute_values = HashMap::new();
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
        .map_err(types::RequestError::Connect)
    });

    match RT.with(|rt| rt.borrow_mut().block_on(result)) {
        Err(err) => {
            log::error!("failed to perform move to seer: {:?}", err);
            send_error(format!("Error moving to seer: {:?}", err),
                event.request_context.connection_id.clone().unwrap(), endpoint(&event.request_context));
        },
        Ok(_) => (),
    };
}

pub fn get_state(table_name: String, event: types::ApiGatewayWebsocketProxyRequest, 
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
        .map(types::RequestResult::Get)
        .map_err(types::RequestError::Get)
    });

    match RT.with(|rt| rt.borrow_mut().block_on(result)) {
        Err(err) => {
            log::error!("failed to find the game: {:?}", err);
            send_error(format!("Lobby not found: {:?}", err),
                event.request_context.connection_id.clone().unwrap(), endpoint(&event.request_context));
        },
        Ok(result) => {
            match result {
                types::RequestResult::Get(result) => {
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
    return None;
}

pub fn check_game_over(players: &Vec<types::Player>) -> bool {
    let good_players: Vec<types::Player> = players.clone().into_iter().filter(|p| p.attributes.as_ref().unwrap().team == types::PlayerTeam::Good).collect();
    let evil_players: Vec<types::Player> = players.clone().into_iter().filter(|p| p.attributes.as_ref().unwrap().team == types::PlayerTeam::Evil).collect();
    return evil_players.len() >= good_players.len() || evil_players.len() == 0;
}
