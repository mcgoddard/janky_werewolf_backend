#[macro_use]
extern crate serde_derive;
use std::collections::HashMap;

use serde::ser::Serialize;
use serde::de::{Deserialize, Deserializer, DeserializeOwned};
use serde_json::Value;
use dynomite::{
    Attributes, Attribute, Item,
};

use aws_lambda_events::event::apigw::ApiGatewayRequestIdentity;

#[derive(Attribute, Deserialize, Serialize, Clone, PartialEq, Debug)]
pub enum PlayerRole {
    Unknown,
    Villager,
    Seer,
    Werewolf,
    Mod,
    Bodyguard,
    Lycan,
    Tanner,
}

#[derive(Attribute, Deserialize, Serialize, Clone, PartialEq, Debug)]
pub enum PlayerTeam {
    Unknown,
    Good,
    Evil,
    Tanner,
    Mod,
}

#[derive(Attributes, Serialize, Deserialize, Clone, Debug)]
pub struct PlayerAttributes {
    pub role: PlayerRole,
    pub team: PlayerTeam,
    pub alive: bool,
    pub visible_to: Vec<String>,
}

#[derive(Attributes, Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub secret: String,
    pub attributes: PlayerAttributes,
}

#[derive(Attribute, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum PhaseName {
    Lobby,
    Day,
    Seer,
    Bodyguard,
    Werewolf,
    End,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct StreamRecord {
    #[serde(rename = "NewImage")]
    pub new_image: Option<HashMap<String, rusoto_dynamodb::AttributeValue>>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(rename = "StreamViewType")]
    pub stream_view_type: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DDBRecord {
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "awsRegion")]
    pub aws_region: Option<String>,
    #[serde(default)]
    pub dynamodb: Option<StreamRecord>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "eventID")]
    pub event_id: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "eventName")]
    pub event_name: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "eventSource")]
    pub event_source: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "eventVersion")]
    pub event_version: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    #[serde(rename = "eventSourceARN")]
    pub event_source_arn: Option<String>,
}


#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DDBStreamEvent {
    #[serde(default)]
    #[serde(rename = "Records")]
    pub records: Option<Vec<DDBRecord>>,
}

#[derive(Attributes, Serialize, Deserialize, Debug, Clone)]
pub struct Phase {
    pub name: PhaseName,
    pub data: HashMap<String, String>,
}

#[derive(Item, Serialize, Deserialize, Debug, Clone)]
pub struct GameState {
    #[serde(rename = "lobbyId")]
    #[dynomite(partition_key)]
    pub lobby_id: String,
    pub phase: Phase,
    pub players: Vec<Player>,
    pub internal_state: HashMap<String, String>,
    pub ttl: u32,
    pub version: u32,
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
    Ok(opt.unwrap_or_default())
}

