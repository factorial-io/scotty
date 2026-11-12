//! The MCP endpoint: transport, authentication, tool scopes and tools.

use axum::http::header;
use axum_test::{TestResponse, TestServer};
use serde_json::{json, Value};

use super::mcp_oauth_tests::{app, assert_challenge, connect, session};
use crate::api::router::ApiRoutes;
use crate::api::test_utils::create_test_app_state_with_settings;

/// Configured bearer token of `identifier:admin` (admin in every scope).
const ADMIN_TOKEN: &str = "test-bearer-token-123";

async fn post(server: &TestServer, token: Option<&str>, body: Value) -> TestResponse {
    let mut request = server
        .post("/mcp")
        .add_header(header::ACCEPT, "application/json, text/event-stream")
        .json(&body);
    if let Some(token) = token {
        request = request.authorization_bearer(token);
    }
    request.await
}

/// One JSON-RPC call; the `result` (or the whole body for an error).
async fn rpc(server: &TestServer, token: &str, method: &str, params: Value) -> Value {
    let response = post(
        server,
        Some(token),
        json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}),
    )
    .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    let body: Value = response.json();
    body.get("result").cloned().unwrap_or(body)
}

fn tool_names(list: &Value) -> Vec<String> {
    let mut names: Vec<String> = list["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();
    names
}

async fn bearer_mode_server() -> TestServer {
    let settings = config::Config::builder()
        .add_source(config::File::with_name("tests/test_bearer_auth"))
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();
    TestServer::new(ApiRoutes::create(
        create_test_app_state_with_settings(settings, None).await,
    ))
}

// --- 2.1 Endpoint and auth ------------------------------------------------------

#[tokio::test]
async fn mcp_and_bearer_tokens_reach_tools_list() {
    let (server, state) = app().await;
    let (_client, tokens) = connect(&server, &state, "alice@example.com", "read", &["read"]).await;
    let mcp_token = tokens["access_token"].as_str().unwrap();

    let init = rpc(
        &server,
        mcp_token,
        "initialize",
        json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "0"},
        }),
    )
    .await;
    assert_eq!(init["serverInfo"]["name"], "scotty", "{init}");
    assert!(init["capabilities"]["tools"].is_object(), "{init}");

    let by_mcp = rpc(&server, mcp_token, "tools/list", json!({})).await;
    let by_bearer = rpc(&server, ADMIN_TOKEN, "tools/list", json!({})).await;
    assert!(by_mcp["tools"].is_array(), "{by_mcp}");
    assert_eq!(tool_names(&by_mcp), tool_names(&by_bearer));
}

#[tokio::test]
async fn bearer_mode_serves_mcp_to_configured_tokens() {
    let server = bearer_mode_server().await;
    let listed = rpc(&server, ADMIN_TOKEN, "tools/list", json!({})).await;
    assert!(listed["tools"].is_array(), "{listed}");
    assert_eq!(post(&server, None, json!({})).await.status_code(), 401);
}

#[tokio::test]
async fn missing_invalid_and_session_tokens_get_the_challenge() {
    let (server, state) = app().await;
    let list = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"});
    assert_challenge(&post(&server, None, list.clone()).await);
    assert_challenge(&post(&server, Some("garbage"), list.clone()).await);
    let alice_session = session(&state, "alice@example.com").await;
    assert_challenge(&post(&server, Some(&alice_session), list).await);
}
