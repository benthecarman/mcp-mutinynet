use crate::mcp::types::*;
use crate::mcp::utilities::{get_bearer_token, get_bearer_token_location};
use rpc_router::HandlerResult;
use url::Url;

pub async fn resources_list(
    _request: Option<ListResourcesRequest>,
) -> HandlerResult<ListResourcesResult> {
    let response = ListResourcesResult {
        resources: vec![Resource {
            uri: Url::parse(&format!(
                "file://{}",
                get_bearer_token_location().to_str().unwrap()
            ))
            .unwrap(),
            name: "github bearer token available".to_string(),
            description: Some(
                "The github bearer token will be available if the user is logged in".to_string(),
            ),
            mime_type: Some("text/plain".to_string()),
        }],
        next_cursor: None,
    };
    Ok(response)
}

pub async fn resource_read(request: ReadResourceRequest) -> HandlerResult<ReadResourceResult> {
    let has_token = get_bearer_token().is_some();
    let response = ReadResourceResult {
        content: ResourceContent {
            uri: request.uri.clone(),
            mime_type: Some("text/plain".to_string()),
            text: Some(has_token.to_string()),
            blob: None,
        },
    };
    Ok(response)
}
