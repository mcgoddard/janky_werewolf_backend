#[macro_use]
extern crate lambda_runtime as lambda;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate simple_logger;
extern crate rand;

use lambda::error::HandlerError;

use std::error::Error;
use std::collections::HashMap;

use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;

use serde_json::{Value, Map};

use simple_logger::SimpleLogger;
use log::LevelFilter;

use common::{types, helpers};

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


#[derive(Deserialize, Serialize, Clone)]
struct RouteEvent {
    action: String,
    #[serde(skip)]
    data: Map<String, Value>,
}

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new().with_level(LevelFilter::Info).init()?;
    lambda!(my_handler);

    Ok(())
}

fn my_handler(e: types::ApiGatewayWebsocketProxyRequest, c: lambda::Context) -> Result<ApiGatewayProxyResponse, HandlerError> {
    let body = e.body.clone().unwrap();
    info!("{:?}", body);
    let event: RouteEvent = serde_json::from_str(&body).unwrap();
    
    match &event.action as &str {
        "bodyguard" => handle_bodyguard(e),
        "join" => handle_join(e, c),
        "lynch" => handle_lynch(e),
        "seer" => handle_seer(e),
        "sleep" => handle_sleep(e),
        "start" => handle_start(e),
        "werewolf" => handle_werewolf(e),
        _ => handle_unknown(event.action, e),
    }
}

fn handle_unknown(action: String, event: types::ApiGatewayWebsocketProxyRequest) -> Result<ApiGatewayProxyResponse, HandlerError> {
    helpers::send_error(format!("Unknown action \"{}\"!", action),
        event.request_context.connection_id.clone().unwrap(), 
        helpers::endpoint(&event.request_context));

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: HashMap::new(),
        multi_value_headers: HashMap::new(),
        body: None,
        is_base64_encoded: None,
    })
}
