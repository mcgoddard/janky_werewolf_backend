use rusoto_apigatewaymanagementapi::{
    ApiGatewayManagementApi, ApiGatewayManagementApiClient, PostToConnectionRequest,
};
use rusoto_core::Region;
use serde_json::json;

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
