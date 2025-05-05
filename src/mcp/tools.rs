use crate::mcp::types::*;
use crate::mcp::utilities;
use maplit::hashmap;
use reqwest::{Client, StatusCode};
use rpc_router::{Handler, HandlerResult, IntoHandlerError, RouterBuilder, RpcParams};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::SystemTime;

/// register all tools to the router
pub fn register_tools(router_builder: RouterBuilder) -> RouterBuilder {
    router_builder
        .append_dyn("tools/list", tools_list.into_dyn())
        .append_dyn("login", login.into_dyn())
        .append_dyn("pay_mutinynet_invoice", pay_mutinynet_invoice.into_dyn())
        .append_dyn("pay_mutinynet_address", pay_mutinynet_address.into_dyn())
}

pub async fn tools_list(_request: Option<ListToolsRequest>) -> HandlerResult<ListToolsResult> {
    let login = Tool {
        name: "login".to_string(),
        description: Some(
            "Authorizes the user so they can use the functionality of the mutinynet MCP server."
                .to_string(),
        ),
        input_schema: ToolInputSchema {
            type_name: "object".to_string(),
            properties: hashmap! {},
            required: vec![],
        },
    };
    let pay_mutinynet_invoice = Tool {
        name: "pay_mutinynet_invoice".to_string(),
        description: Some("Pays the given mutinynet invoice".to_string()),
        input_schema: ToolInputSchema {
            type_name: "object".to_string(),
            properties: hashmap! {
                "invoice".to_string() => ToolInputSchemaProperty {
                    type_name: Some("string".to_owned()),
                    description: Some("Mutinynet invoice to pay".to_owned()),
                    enum_values: None,
                }
            },
            required: vec!["invoice".to_string()],
        },
    };
    let pay_mutinynet_address = Tool {
        name: "pay_mutinynet_address".to_string(),
        description: Some("Pays the given mutinynet address".to_string()),
        input_schema: ToolInputSchema {
            type_name: "object".to_string(),
            properties: hashmap! {
                "address".to_string() => ToolInputSchemaProperty {
                    type_name: Some("string".to_owned()),
                    description: Some("Mutinynet address to pay".to_owned()),
                    enum_values: None,
                },
                "amount".to_string() => ToolInputSchemaProperty {
                    type_name: Some("number".to_owned()),
                    description: Some("The amount in satoshis to pay the address, if none is given 5k sats will be used".to_owned()),
                    enum_values: None,
                },
            },
            required: vec!["address".to_string()],
        },
    };
    let response = ListToolsResult {
        tools: vec![login, pay_mutinynet_invoice, pay_mutinynet_address],
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

#[derive(Deserialize)]
struct DeviceReturn {
    token: String,
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
                    if let Ok(res) = client
                        .post("https://faucet.mutinynet.com/auth/github/device")
                        .json(&json!({
                            "code": res.access_token,
                        }))
                        .header("Content-Type", "application/json")
                        .send()
                        .await
                    {
                        if let Ok(device) = res.json::<DeviceReturn>().await {
                            utilities::write_bearer_token(device.token);
                            return;
                        }
                    }
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

#[derive(Deserialize, Serialize, RpcParams)]
pub struct PayInvoiceRequest {
    invoice: String,
}

#[derive(Deserialize)]
pub struct LightningResponse {
    pub payment_hash: String,
}

pub async fn pay_mutinynet_invoice(req: PayInvoiceRequest) -> HandlerResult<CallToolResult> {
    let token = match utilities::get_bearer_token() {
        Some(token) => token,
        None => {
            return login(LoginRequest {}).await;
        }
    };

    let client = Client::new();

    let resp = client
        .post("https://faucet.mutinynet.com/api/lightning")
        .json(&json!({
            "bolt11": req.invoice,
        }))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|_| {
            json!({"code": -32603, "message": "Error making request"}).into_handler_error()
        })?;

    let status = resp.status();
    if !status.is_success() {
        if status == StatusCode::from_u16(401).unwrap() {
            return login(LoginRequest {}).await;
        }

        let text = resp.text().await.map_err(|_| {
            json!({"code": -32603, "message": "Error decoding text"}).into_handler_error()
        })?;

        return Err(
            json!({"code": -32603, "message": format!("Error ({status}): {text}")})
                .into_handler_error(),
        );
    }

    let res: LightningResponse = resp.json().await.map_err(|_| {
        json!({"code": -32603, "message": "Error decoding response"}).into_handler_error()
    })?;

    let text = format!("Payment success! Preimage: {}", res.payment_hash);
    Ok(CallToolResult {
        is_error: false,
        content: vec![CallToolResultContent::Text { text }],
    })
}

#[derive(Deserialize, Serialize, RpcParams)]
pub struct PayAddressRequest {
    address: String,
    amount: Option<u64>,
}

#[derive(Deserialize)]
pub struct OnChainResponse {
    pub txid: String,
}

pub async fn pay_mutinynet_address(req: PayAddressRequest) -> HandlerResult<CallToolResult> {
    let token = match utilities::get_bearer_token() {
        Some(token) => token,
        None => {
            return login(LoginRequest {}).await;
        }
    };
    let amount = req.amount.unwrap_or(5_000);

    if amount > 1_000_00 {
        let text = "Amount is too high, max send amount is 1,000,000 sats".to_string();
        return Ok(CallToolResult {
            is_error: true,
            content: vec![CallToolResultContent::Text { text }],
        });
    }

    let client = Client::new();

    let resp = client
        .post("https://faucet.mutinynet.com/api/onchain")
        .json(&json!({
            "sats": amount,
            "address": req.address,
        }))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|_| {
            json!({"code": -32603, "message": "Error making request"}).into_handler_error()
        })?;

    let status = resp.status();
    if !status.is_success() {
        if status == StatusCode::from_u16(401).unwrap() {
            return login(LoginRequest {}).await;
        }

        let text = resp.text().await.map_err(|_| {
            json!({"code": -32603, "message": "Error decoding text"}).into_handler_error()
        })?;

        return Err(
            json!({"code": -32603, "message": format!("Error ({status}): {text}")})
                .into_handler_error(),
        );
    }

    let res: OnChainResponse = resp.json().await.map_err(|_| {
        json!({"code": -32603, "message": "Error decoding response"}).into_handler_error()
    })?;

    let text = format!("Payment success! Transaction id: {}", res.txid);
    Ok(CallToolResult {
        is_error: false,
        content: vec![CallToolResultContent::Text { text }],
    })
}
