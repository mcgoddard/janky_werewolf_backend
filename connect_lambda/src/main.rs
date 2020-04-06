#[macro_use]
extern crate lambda_runtime as lambda;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate simple_logger;

use lambda::error::HandlerError;

use std;
use std::{cell::RefCell, env};
use std::error::Error;

use std::collections::HashMap;

use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;

use dynomite::{
    dynamodb::{
        DynamoDb, DynamoDbClient, PutItemError, PutItemInput,
    },
};
use futures::Future;
use rusoto_core::RusotoError;
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
}

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::init_with_level(log::Level::Info)?;
    lambda!(my_handler);

    Ok(())
}

fn my_handler(e: types::ApiGatewayWebsocketProxyRequest, c: lambda::Context) -> Result<ApiGatewayProxyResponse, HandlerError> {
    let body = e.body.unwrap();
    print!("{:?}", body);
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
        None => new_game(p.name, p.secret, "AAAA".to_string(), e.request_context.connection_id.unwrap()),
        _ => (), // Some(c)
    };

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: HashMap::new(),
        multi_value_headers: HashMap::new(),
        body: None,
        is_base64_encoded: None,
    })
}

fn new_game(name: String, secret: String, code: String, connection_id: String) {
    let table_name = env::var("tableName").unwrap();
    let item = types::GameState {
        lobby_id: code,
        phase: types::Phase {
            name: types::PhaseName::Lobby,
            data: HashMap::new(),
        },
        players: vec![types::Player{
            id: connection_id,
            name: name,
            secret: secret,
            attributes: None,
        }]
    };
    let mut item_hashmap = HashMap::new();
    item_hashmap.insert("lobby_id".to_string(), item.lobby_id);
    item_hashmap.insert("version".to_string(), 1.to_string());
    item_hashmap.insert("data".to_string(), json!(item).to_string());
    let result = DDB.with(|ddb| {
        ddb.put_item(PutItemInput {
            table_name,
            item: item_hashmap,
            ..PutItemInput::default()
        })
        .map(drop)
        .map_err(RequestError::Connect)
    });

    if let Err(err) = RT.with(|rt| rt.borrow_mut().block_on(result)) {
        log::error!("failed to perform new game connection operation: {:?}", err);
    }
}
