#[macro_use]
extern crate lambda_runtime as lambda;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate simple_logger;
extern crate rand;

use lambda::error::HandlerError;

use std;
use std::{cell::RefCell, env};
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;
use rusoto_apigatewaymanagementapi::{
    ApiGatewayManagementApi, ApiGatewayManagementApiClient, PostToConnectionRequest,
};

use dynomite::{
    dynamodb::{
        DynamoDb, DynamoDbClient, PutItemError, PutItemInput, AttributeValue, GetItemInput, GetItemError, GetItemOutput,
    },
};
use futures::Future;
use rand::Rng;
use rusoto_core::{RusotoError, Region};
use tokio::runtime::Runtime;
use serde_json::json;

mod types;

thread_local!(
    static DDB: DynamoDbClient = DynamoDbClient::new(Default::default());
);

thread_local!(
    static RT: RefCell<Runtime> =
        RefCell::new(Runtime::new().expect("failed to initialize runtime"));
);

#[derive(Deserialize, Serialize, Clone)]
struct ConnectEvent {
    action: String,
    name: String,
    secret: String,
    code: Option<String>,
}

#[derive(Serialize, Clone)]
struct CustomOutput {
    message: String,
}

#[derive(Debug)]
enum RequestError {
    Connect(RusotoError<PutItemError>),
    Get(RusotoError<GetItemError>),
}

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::init_with_level(log::Level::Info)?;
    lambda!(my_handler);

    Ok(())
}

fn my_handler(e: types::ApiGatewayWebsocketProxyRequest, c: lambda::Context) -> Result<ApiGatewayProxyResponse, HandlerError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let p: ConnectEvent = serde_json::from_str(&body).unwrap();
    
    if p.name == "" {
        error!("Empty name in request {}", c.aws_request_id);
        return Err(c.new_error("Empty first name"));
    }
    else if p.secret == "" {
        error!("Empty secret in request {}", c.aws_request_id);
        return Err(c.new_error("Empty secret"));
    }

    match p.code {
        None => new_game(e, p.name, p.secret),
        Some(c) => join_game(e, p.name, p.secret, p.code.unwrap()),
    };

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: HashMap::new(),
        multi_value_headers: HashMap::new(),
        body: None,
        is_base64_encoded: None,
    })
}

fn endpoint(ctx: &types::ApiGatewayWebsocketProxyRequestContext) -> String {
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

fn new_game(event: types::ApiGatewayWebsocketProxyRequest, name: String, secret: String) {
    let table_name = env::var("tableName").unwrap();

    let mut rng = rand::thread_rng();
    let valid_code_chars = vec!["A","B","C","D","E","F","G","H","I","J","K","L","M","N","O","P","Q","R","S","T","U","V","W","X","Y","Z"];
    let code: String = (0..4).map(|_| valid_code_chars[rng.gen_range(0, 26) as usize].clone()).collect();

    let item = types::GameState {
        lobby_id: code,
        phase: types::Phase {
            name: types::PhaseName::Lobby,
            data: HashMap::new(),
        },
        players: vec![types::Player{
            id: event.request_context.connection_id.clone().unwrap(),
            name: name,
            secret: secret,
            attributes: None,
        }]
    };
    let mut item_hashmap = HashMap::new();
    item_hashmap.insert("lobby_id".to_string(), AttributeValue {
        s: Some(item.lobby_id.clone()),
        ..Default::default()
    });
    item_hashmap.insert("version".to_string(), AttributeValue {
        n: Some(1.to_string()),
        ..Default::default()
    });
    let data = json!(item);
    item_hashmap.insert("data".to_string(), AttributeValue {
        s: Some(data.to_string()),
        ..Default::default()
    });
    let since_the_epoch = SystemTime::now().duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    item_hashmap.insert("ttl".to_string(), AttributeValue {
        n: Some(format!("{}", (since_the_epoch.as_secs() as i32) + (48*60*60))),
        ..Default::default()
    });
    let result = DDB.with(|ddb| {
        ddb.put_item(PutItemInput {
            table_name,
            condition_expression: Some("attribute_not_exists(lobby_id)".to_string()),
            item: item_hashmap,
            ..PutItemInput::default()
        })
        .map(drop)
        .map_err(RequestError::Connect)
    });

    match RT.with(|rt| rt.borrow_mut().block_on(result)) {
        Err(err) => {
            log::error!("failed to perform new game connection operation: {:?}", err);
            let client = ApiGatewayManagementApiClient::new(Region::Custom {
                name: Region::EuWest2.name().into(),
                endpoint: endpoint(&event.request_context),
            });
            let result = client.clone().post_to_connection(PostToConnectionRequest {
                            connection_id: event.request_context.connection_id.unwrap(),
                            data: serde_json::to_vec(&json!({ "message": format!("{:?}", err) })).unwrap_or_default(),
                        }).sync();
            match result {
                Err(e) => error!("{:?}", e),
                _ => (),
            }
        },
        Ok(_) => (),
    };
}

fn join_game(event: types::ApiGatewayWebsocketProxyRequest, name: String, secret: String, lobby_id: String) {
    let table_name = env::var("tableName").unwrap();

    let mut ddb_keys = HashMap::new();
    ddb_keys.insert("lobby_id".to_string(), AttributeValue {
        s: Some(lobby_id.to_string()),
        ..Default::default()
    });

    let result = DDB.with(|ddb| {
        ddb.get_item(GetItemInput {
            table_name,
            key: ddb_keys,
            ..GetItemInput::default()
        })
        .map(drop)
        .map_err(RequestError::Get)
    });

    match RT.with(|rt| rt.borrow_mut().block_on(result)) {
        Err(err) => {
            log::error!("failed to perform new game connection operation: {:?}", err);
            let client = ApiGatewayManagementApiClient::new(Region::Custom {
                name: Region::EuWest2.name().into(),
                endpoint: endpoint(&event.request_context),
            });
            let result = client.clone().post_to_connection(PostToConnectionRequest {
                            connection_id: event.request_context.connection_id.unwrap(),
                            data: serde_json::to_vec(&json!({ "message": format!("{:?}", err) })).unwrap_or_default(),
                        }).sync();
            match result {
                Err(e) => error!("{:?}", e),
                _ => (),
            }
        },
        Ok(result) => {
            let result: GetItemOutput = result;
            match result.item {
                None => {
                    error!("Lobby not found: {:?}", lobby_id);
                    let client = ApiGatewayManagementApiClient::new(Region::Custom {
                        name: Region::EuWest2.name().into(),
                        endpoint: endpoint(&event.request_context),
                    });
                    let result = client.clone().post_to_connection(PostToConnectionRequest {
                                    connection_id: event.request_context.connection_id.unwrap(),
                                    data: serde_json::to_vec(&json!({ "message": "Unable to find lobby" })).unwrap_or_default(),
                                }).sync();
                    match result {
                        Err(e) => error!("{:?}", e),
                        _ => (),
                    }
                },
                Some(item) => {
                    let mut data: types::GameState = serde_json::from_str(&item["data"].s.unwrap()).unwrap();
                    let existing_player = data.players.into_iter().filter(|player| player.name == name).collect();
                    if existing_player.len() == 1 {
                        if existing_player[0].secret == secret {
                            let mut new_players = data.players.retain(|player| player.name != name);
                            new_players.push(types::Player{
                                id: event.request_context.connection_id.clone().unwrap(),
                                name: name,
                                secret: secret,
                                attributes: None,
                            });
                            data.players = new_players;
                        }
                        else {
                            error!("Non-matching secret for {:?}", name);
                        }
                    }
                    else {
                        data.players.push(types::Player{
                            id: event.request_context.connection_id.clone().unwrap(),
                            name: name,
                            secret: secret,
                            attributes: None,
                        });
                    }
                    let mut new_item = item.clone();
                    new_item["version"] = AttributeValue {
                        n: Some(format!("{}", new_item["version"].n.unwrap().parse::<i32>().unwrap() + 1)),
                        ..Default::default()
                    };
                    new_item["data"] = AttributeValue {
                        s: Some(data.to_string()),
                        ..Default::default()
                    };
                    let result = DDB.with(|ddb| {
                        ddb.put_item(PutItemInput {
                            table_name,
                            condition_expression: Some("attribute_not_exists(lobby_id)".to_string()),
                            item: new_item,
                            ..PutItemInput::default()
                        })
                        .map(drop)
                        .map_err(RequestError::Connect)
                    });
                
                    match RT.with(|rt| rt.borrow_mut().block_on(result)) {
                        Err(err) => {
                            log::error!("failed to perform new game connection operation: {:?}", err);
                            let client = ApiGatewayManagementApiClient::new(Region::Custom {
                                name: Region::EuWest2.name().into(),
                                endpoint: endpoint(&event.request_context),
                            });
                            let result = client.clone().post_to_connection(PostToConnectionRequest {
                                            connection_id: event.request_context.connection_id.unwrap(),
                                            data: serde_json::to_vec(&json!({ "message": format!("{:?}", err) })).unwrap_or_default(),
                                        }).sync();
                            match result {
                                Err(e) => error!("{:?}", e),
                                _ => (),
                            }
                        },
                        Ok(_) => (),
                    };
                }
            }
        },
    };
}
