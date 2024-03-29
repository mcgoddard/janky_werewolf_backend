extern crate lambda;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate simple_logger;
extern crate rand;
extern crate tokio;
extern crate rusoto_core;
extern crate rusoto_dynamodb;
extern crate serde_dynamodb;

use lambda::{handler_fn, Context};

use std::fmt;
use std::error::Error;
use std::collections::HashMap;

use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;

use serde_json::{Value, Map};
use simple_logger::SimpleLogger;
use log::LevelFilter;

mod bodyguard;
use bodyguard::handle_bodyguard;

mod join;
use join::handle_join;

mod lynch;
use lynch::handle_lynch;

mod seer;
use seer::handle_seer;

mod sleep;
use sleep::handle_sleep;

mod start;
use start::handle_start;

mod werewolf;
use werewolf::handle_werewolf;

mod helpers;


#[derive(Deserialize, Serialize, Clone)]
struct RouteEvent {
    action: String,
    #[serde(skip)]
    data: Map<String, Value>,
}

type LambdaError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
    SimpleLogger::new().with_level(LevelFilter::Info).init()?;
    let func = handler_fn(handler);
    lambda::run(func).await?;
    Ok(())
}

async fn handler(e: common::ApiGatewayWebsocketProxyRequest, c: Context) -> Result<ApiGatewayProxyResponse, LambdaError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let event: RouteEvent = serde_json::from_str(&body).unwrap();
    
    let error = match &event.action as &str {
        "bodyguard" => handle_bodyguard(e.clone()),
        "join" => handle_join(e.clone(), c),
        "lynch" => handle_lynch(e.clone()),
        "seer" => handle_seer(e.clone()),
        "sleep" => handle_sleep(e.clone()),
        "start" => handle_start(e.clone()),
        "werewolf" => handle_werewolf(e.clone()),
        _ => handle_unknown(event.action),
    };

    if let Err(action_error) = error {
        helpers::send_error(format!("Unknown action \"{}\"!", action_error),
            e.clone().request_context.connection_id.unwrap(),
            helpers::endpoint(&e.request_context));
    }

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: HashMap::new(),
        multi_value_headers: HashMap::new(),
        body: None,
        is_base64_encoded: None,
    })
}

fn handle_unknown(action: String) -> Result<(), ActionError> {
    Err(ActionError::new(&format!("Unknown action \"{}\"!", action)))
}

#[derive(Debug)]
pub struct ActionError {
    details: String
}

impl ActionError {
    fn new(msg: &str) -> ActionError {
        ActionError{details: msg.to_string()}
    }
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self.details)
    }
}

impl Error for ActionError {
    fn description(&self) -> &str {
        &self.details
    }
}
