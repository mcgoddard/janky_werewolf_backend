#[macro_use]
extern crate lambda_runtime as lambda;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate simple_logger;

use lambda::error::HandlerError;

use std::error::Error;

use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use serde_json::Value;
use std::collections::HashMap;

use serde;
use serde::de::{Deserialize, Deserializer};
use serde::ser::Serializer;
use std;

use aws_lambda_events::event::apigw::ApiGatewayRequestIdentity;
use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;

#[derive(Deserialize, Serialize, Clone)]
struct CustomEvent {
    action: String,
    #[serde(rename = "firstName")]
    first_name: String,
}

#[derive(Serialize, Clone)]
struct CustomOutput {
    message: String,
}

#[cfg(not(feature = "string-null-empty"))]
pub(crate) fn deserialize_lambda_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::deserialize(deserializer)? {
        Some(s) => {
            if s == "" {
                Ok(None)
            } else {
                Ok(Some(s))
            }
        }
        None => Ok(None),
    }
}

pub(crate) fn deserialize_lambda_map<'de, D, K, V>(
    deserializer: D,
) -> Result<HashMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: serde::Deserialize<'de>,
    K: std::hash::Hash,
    K: std::cmp::Eq,
    V: serde::Deserialize<'de>,
{
    // https://github.com/serde-rs/serde/issues/1098
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or(HashMap::default()))
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ApiGatewayWebsocketProxyRequest {
    /// The resource path defined in API Gateway
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub resource: Option<String>,
    /// The url path for the caller
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub path: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "httpMethod")]
    pub http_method: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_map")]
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(deserialize_with = "deserialize_lambda_map")]
    #[serde(default)]
    #[serde(rename = "multiValueHeaders")]
    pub multi_value_headers: HashMap<String, Vec<String>>,
    #[serde(deserialize_with = "deserialize_lambda_map")]
    #[serde(default)]
    #[serde(rename = "queryStringParameters")]
    pub query_string_parameters: HashMap<String, String>,
    #[serde(deserialize_with = "deserialize_lambda_map")]
    #[serde(default)]
    #[serde(rename = "multiValueQueryStringParameters")]
    pub multi_value_query_string_parameters: HashMap<String, Vec<String>>,
    #[serde(deserialize_with = "deserialize_lambda_map")]
    #[serde(default)]
    #[serde(rename = "pathParameters")]
    pub path_parameters: HashMap<String, String>,
    #[serde(deserialize_with = "deserialize_lambda_map")]
    #[serde(default)]
    #[serde(rename = "stageVariables")]
    pub stage_variables: HashMap<String, String>,
    #[serde(rename = "requestContext")]
    pub request_context: ApiGatewayWebsocketProxyRequestContext,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub body: Option<String>,
    #[serde(rename = "isBase64Encoded")]
    pub is_base64_encoded: Option<bool>,
}


fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::init_with_level(log::Level::Info)?;
    lambda!(my_handler);

    Ok(())
}

fn my_handler(e: ApiGatewayWebsocketProxyRequest, c: lambda::Context) -> Result<ApiGatewayProxyResponse, HandlerError> {
    let body = e.body.unwrap();
    print!("{:?}", body);
    let p: CustomEvent = serde_json::from_str(&body).unwrap();
    
    if p.first_name == "" {
        error!("Empty first name in request {}", c.aws_request_id);
        return Err(c.new_error("Empty first name"));
    }

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: HashMap::new(),
        multi_value_headers: HashMap::new(),
        body: Some(format!("Hello, {}!", p.first_name)),
        is_base64_encoded: Some(false),
    })
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ApiGatewayWebsocketProxyRequestContext<T1 = Value, T2 = Value>
where
    T1: DeserializeOwned,
    T1: Serialize,
    T2: DeserializeOwned,
    T2: Serialize,
{
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "accountId")]
    pub account_id: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "resourceId")]
    pub resource_id: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub stage: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
    pub identity: ApiGatewayRequestIdentity,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "resourcePath")]
    pub resource_path: Option<String>,
    #[serde(bound = "")]
    pub authorizer: Option<T1>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "httpMethod")]
    pub http_method: Option<String>,
    /// The API Gateway rest API Id
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "apiId")]
    pub apiid: Option<String>,
    #[serde(rename = "connectedAt")]
    pub connected_at: i64,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "connectionId")]
    pub connection_id: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "domainName")]
    pub domain_name: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub error: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "eventType")]
    pub event_type: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "extendedRequestId")]
    pub extended_request_id: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "integrationLatency")]
    pub integration_latency: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "messageDirection")]
    pub message_direction: Option<String>,
    #[serde(bound = "")]
    #[serde(rename = "messageId")]
    pub message_id: T2,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "requestTime")]
    pub request_time: Option<String>,
    #[serde(rename = "requestTimeEpoch")]
    pub request_time_epoch: i64,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "routeKey")]
    pub route_key: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub status: Option<String>,
}
