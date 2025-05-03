use crate::mcp::types::*;
use crate::mcp::utilities;
use maplit::hashmap;
use reqwest::Client;
use rpc_router::{Handler, HandlerResult, IntoHandlerError, RouterBuilder, RpcParams};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::SystemTime;

/// register all tools to the router
pub fn register_tools(router_builder: RouterBuilder) -> RouterBuilder {
    router_builder
        .append_dyn("tools/list", tools_list.into_dyn())
        .append_dyn("login", login.into_dyn())
}

pub async fn tools_list(_request: Option<ListToolsRequest>) -> HandlerResult<ListToolsResult> {
    let response = ListToolsResult {
		tools: vec![Tool {
			name: "login".to_string(),
			description: Some("Authorizes the user so they can use the functionality of the mutinynet MCP server.".to_string()),
			input_schema: ToolInputSchema {
				type_name: "object".to_string(),
				properties: hashmap! { },
				required: vec![],
			},
		}],
		next_cursor: None,
	};
    Ok(response)
}

const GH_CLIENT_ID: &str = "Ov23liIa6qfR9KtYHwUF";
const GH_SCOPE: &str = "user:email";

#[derive(Deserialize, Serialize, RpcParams)]
pub struct LoginRequest {}

#[derive(Deserialize)]
struct DeviceLoginResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Deserialize)]
struct AccessTokenResponse {
    access_token: String,
}

pub async fn login(_: LoginRequest) -> HandlerResult<CallToolResult> {
    let client = Client::new();
    let resp: DeviceLoginResponse = client
        .post("https://github.com/login/device/code")
        .json(&json!({
            "client_id": GH_CLIENT_ID,
            "scope": GH_SCOPE,
        }))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|_| json!({"code": -32603, "message": "Internal error"}).into_handler_error())?
        .json()
        .await
        .map_err(|_| json!({"code": -32603, "message": "Internal error"}).into_handler_error())?;

    let start = SystemTime::now();

    let text = format!(
        "Open this url: {} and enter the code: {} to login",
        resp.verification_uri, resp.user_code
    );

    // spawn thread to watch for oauth login
    tokio::spawn(async move {
        let json = json!({
            "client_id": GH_CLIENT_ID,
            "device_code": resp.device_code,
            "grant_type": "urn:ietf:params:oauth:grant-type:device_code"
        });
        while SystemTime::now().duration_since(start).unwrap().as_secs() < resp.expires_in + 10 {
            if let Ok(res) = client
                .post("https://github.com/login/oauth/access_token")
                .json(&json)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .send()
                .await
            {
                if let Ok(res) = res.json::<AccessTokenResponse>().await {
                    utilities::write_bearer_token(res.access_token);
                    return;
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(resp.interval)).await;
        }
    });

    Ok(CallToolResult {
        is_error: false,
        content: vec![CallToolResultContent::Text { text }],
    })
}
