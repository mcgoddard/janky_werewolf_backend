#[macro_use]
extern crate lambda_runtime as lambda;
#[macro_use]
extern crate serde_derive;
extern crate simple_logger;

use lambda::error::HandlerError;

use std;
use std::error::Error;
use std::collections::HashMap;

use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;
use aws_lambda_events::event::sqs::SqsEvent;

mod types;

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::init_with_level(log::Level::Info)?;
    lambda!(my_handler);

    Ok(())
}

fn my_handler(e: SqsEvent, _c: lambda::Context) -> Result<ApiGatewayProxyResponse, HandlerError> {
    print!("{:?}", e);
    // print!("{}", json!(e).to_string());
    // match &e.records[0].body {
    //     Some(m) => print!("{:?}", m),
    //     None => print!("No message..."),
    // };

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: HashMap::new(),
        multi_value_headers: HashMap::new(),
        body: None,
        is_base64_encoded: None,
    })
}
