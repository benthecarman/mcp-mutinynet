use crate::mcp::types::*;
use crate::mcp::{PROTOCOL_VERSION, SERVER_NAME, SERVER_VERSION};
use rpc_router::HandlerResult;
use serde_json::{Value, json};
use std::path::PathBuf;

/// handler for `initialize` request from client
pub async fn initialize(_request: InitializeRequest) -> HandlerResult<InitializeResult> {
    let result = InitializeResult {
        protocol_version: PROTOCOL_VERSION.to_string(),
        server_info: Implementation {
            name: SERVER_NAME.to_string(),
            version: SERVER_VERSION.to_string(),
        },
        capabilities: ServerCapabilities {
            experimental: None,
            prompts: Some(PromptCapabilities::default()),
            resources: None,
            tools: Some(json!({})),
            roots: None,
            sampling: None,
            logging: None,
        },
        instructions: None,
    };
    Ok(result)
}

/// handler for SIGINT by client
pub fn graceful_shutdown() {
    // shutdown server
}

/// handler for `notifications/initialized` from client
pub fn notifications_initialized() {}

/// handler for `notifications/cancelled` from client
pub fn notifications_cancelled(_params: CancelledNotification) {
    // cancel request
}

pub async fn ping(_request: PingRequest) -> HandlerResult<EmptyResult> {
    Ok(EmptyResult {})
}

pub async fn logging_set_level(_request: SetLevelRequest) -> HandlerResult<LoggingResponse> {
    Ok(LoggingResponse {})
}

pub async fn roots_list(_request: Option<ListRootsRequest>) -> HandlerResult<ListRootsResult> {
    let response = ListRootsResult {
        roots: vec![Root {
            name: "my project".to_string(),
            url: "file:///home/user/projects/my-project".to_string(),
        }],
    };
    Ok(response)
}

/// send notification to client
pub fn notify(method: &str, params: Option<Value>) {
    let notification = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    });
    println!("{}", serde_json::to_string(&notification).unwrap());
}

pub fn get_bearer_token_location() -> PathBuf {
    home::home_dir()
        .expect("No Home Directory found")
        .join(".config/mcp/mutinynet/gh-token")
}

pub fn get_bearer_token() -> Option<String> {
    let loc = get_bearer_token_location();
    if loc.exists() {
        Some(std::fs::read_to_string(loc).ok()?.trim().to_string())
    } else {
        None
    }
}

pub fn write_bearer_token(bearer_token: String) {
    let loc = get_bearer_token_location();
    std::fs::create_dir_all(loc.parent().unwrap()).unwrap();
    if !loc.exists() {
        std::fs::File::create(&loc).unwrap();
    }
    std::fs::write(loc, bearer_token.as_bytes()).unwrap();
}
