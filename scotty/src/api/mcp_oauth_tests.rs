//! The MCP authorization server end to end (openspec adopt-grantry, tasks
//! 4.1 to 4.4): discovery, registration, authorize, consent, token, `/mcp`
//! and the connections API.

use std::sync::Arc;

use axum::http::header;
use axum_test::{TestResponse, TestServer};
use scotty_core::authorization::Permission;
use serde_json::{json, Value};

use crate::api::router::ApiRoutes;
use crate::api::test_utils::{
    create_test_app_state_with_policy_dir, create_test_app_state_with_settings,
};
use crate::app_state::AppState;
use crate::oauth::mcp::{McpPrincipal, McpScope, McpTarget};

const BASE: &str = "http://localhost:21342";
const RESOURCE: &str = "http://localhost:21342/mcp";
// RFC 7636 appendix B.
const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
const CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
const LOOPBACK: &str = "http://127.0.0.1:3000/callback";
const HTTPS_REDIRECT: &str = "https://app.example/cb";
pub(super) const ADMIN: &str = "admin@example.com";

pub(super) fn settings(overrides: &[(&str, &str)]) -> crate::settings::config::Settings {
    let mut builder =
        config::Config::builder().add_source(config::File::with_name("tests/test_oauth_auth"));
    for (key, value) in overrides {
        builder = builder.set_override(*key, *value).unwrap();
    }
    builder.build().unwrap().try_deserialize().unwrap()
}

/// oauth mode, with `admin@example.com` an admin in a temporary policy copy.
pub(super) async fn app() -> (TestServer, Arc<AppState>) {
    let dir = tempfile::tempdir().unwrap().keep();
    let policy = std::fs::read_to_string("tests/fixtures/policy/policy.yaml")
        .unwrap()
        .replace(
            "assignments:\n",
            &format!(
                "assignments:\n  {ADMIN}:\n    - role: admin\n      scopes:\n        - default\n"
            ),
        );
    std::fs::write(dir.join("policy.yaml"), policy).unwrap();
    let state =
        create_test_app_state_with_policy_dir(settings(&[]), None, dir.to_str().unwrap()).await;
    (TestServer::new(ApiRoutes::create(state.clone())), state)
}

fn bearer(token: &str) -> axum::http::HeaderValue {
    format!("Bearer {token}").parse().unwrap()
}

fn location(response: &TestResponse) -> String {
    response
        .headers()
        .get(header::LOCATION)
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default()
}

fn query_param(url: &str, key: &str) -> Option<String> {
    url::Url::parse(url)
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.into_owned())
}

pub(super) async fn session(state: &AppState, email: &str) -> String {
    state
        .session_server
        .as_ref()
        .unwrap()
        .issue_grant(email.to_string(), "scotty-web", "Scotty web", "api")
        .await
        .unwrap()
        .1
        .access_token
}

async fn register(server: &TestServer, ip: &str) -> TestResponse {
    server
        .post("/oauth/mcp/register")
        .add_header("x-forwarded-for", ip)
        .json(&json!({
            "client_name": "Test Client",
            "redirect_uris": [LOOPBACK, HTTPS_REDIRECT],
            "grant_types": ["authorization_code", "refresh_token"],
        }))
        .await
}

async fn client(server: &TestServer) -> String {
    let response = register(server, "192.0.2.10").await;
    assert_eq!(response.status_code(), 201, "{}", response.text());
    response.json::<Value>()["client_id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn authorize_query(client_id: &str, redirect_uri: &str, scope: &str, pkce: bool) -> String {
    let mut q = url::form_urlencoded::Serializer::new(String::new());
    q.append_pair("response_type", "code")
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", scope)
        .append_pair("state", "xyz")
        .append_pair("resource", RESOURCE);
    if pkce {
        q.append_pair("code_challenge", CHALLENGE)
            .append_pair("code_challenge_method", "S256");
    }
    q.finish()
}

async fn authorize(server: &TestServer, query: &str) -> TestResponse {
    server.get(&format!("/oauth/mcp/authorize?{query}")).await
}

/// Authorize and return the pending id from the consent redirect.
async fn pending(server: &TestServer, client_id: &str, redirect_uri: &str, scope: &str) -> String {
    let response = authorize(
        server,
        &authorize_query(client_id, redirect_uri, scope, true),
    )
    .await;
    let to = location(&response);
    let id = to
        .strip_prefix(&format!("{BASE}/oauth/consent/"))
        .unwrap_or_else(|| panic!("not a consent redirect: {to} ({})", response.text()));
    id.to_string()
}

async fn consent(
    server: &TestServer,
    token: &str,
    id: &str,
    action: &str,
    body: Value,
) -> TestResponse {
    server
        .post(&format!(
            "/api/v1/authenticated/oauth/consent/{id}/{action}"
        ))
        .add_header(header::AUTHORIZATION, bearer(token))
        .json(&body)
        .await
}

async fn exchange(server: &TestServer, client_id: &str, code: &str) -> TestResponse {
    server
        .post("/oauth/mcp/token")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", client_id),
            ("redirect_uri", LOOPBACK),
            ("code_verifier", VERIFIER),
            ("resource", RESOURCE),
        ])
        .await
}

async fn refresh(server: &TestServer, client_id: &str, refresh_token: &str) -> TestResponse {
    server
        .post("/oauth/mcp/token")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", client_id),
        ])
        .await
}

/// Register, authorize, approve as `email` with `kept`, exchange: the tokens.
pub(super) async fn connect(
    server: &TestServer,
    state: &AppState,
    email: &str,
    scope: &str,
    kept: &[&str],
) -> (String, Value) {
    let client_id = client(server).await;
    let id = pending(server, &client_id, LOOPBACK, scope).await;
    let user = session(state, email).await;
    let approved = consent(server, &user, &id, "approve", json!({ "scopes": kept })).await;
    assert_eq!(approved.status_code(), 200, "{}", approved.text());
    let redirect = approved.json::<Value>()["redirect"]
        .as_str()
        .unwrap()
        .to_string();
    let code = query_param(&redirect, "code").unwrap();
    let tokens = exchange(server, &client_id, &code).await;
    assert_eq!(tokens.status_code(), 200, "{}", tokens.text());
    (client_id, tokens.json())
}

async fn mcp(server: &TestServer, token: Option<&str>) -> TestResponse {
    let mut request = server
        .post("/mcp")
        .add_header(header::ACCEPT, "application/json, text/event-stream")
        .json(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}));
    if let Some(token) = token {
        request = request.add_header(header::AUTHORIZATION, bearer(token));
    }
    request.await
}

pub(super) fn assert_challenge(response: &TestResponse) {
    assert_eq!(response.status_code(), 401);
    let challenge = response
        .headers()
        .get(header::WWW_AUTHENTICATE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        challenge.starts_with(&format!(
            "Bearer resource_metadata=\"{BASE}/.well-known/oauth-protected-resource/mcp\""
        )),
        "{challenge}"
    );
}

// --- 4.1 Discovery, registration, authorize, token ----------------------------

#[tokio::test]
async fn metadata_documents_come_from_the_base_url() {
    let (server, _state) = app().await;
    let doc: Value = server
        .get("/.well-known/oauth-authorization-server")
        .add_header(header::HOST, "evil.example")
        .await
        .json();
    assert_eq!(doc["issuer"], BASE);
    assert_eq!(
        doc["authorization_endpoint"],
        format!("{BASE}/oauth/mcp/authorize")
    );
    assert_eq!(doc["token_endpoint"], format!("{BASE}/oauth/mcp/token"));
    assert_eq!(
        doc["registration_endpoint"],
        format!("{BASE}/oauth/mcp/register")
    );
    assert_eq!(doc["code_challenge_methods_supported"], json!(["S256"]));
    assert_eq!(doc["client_id_metadata_document_supported"], true);
    assert_eq!(
        doc["scopes_supported"],
        json!(["read", "create", "destroy", "offline_access"])
    );

    for path in [
        "/.well-known/oauth-protected-resource/mcp",
        "/.well-known/oauth-protected-resource",
    ] {
        let doc: Value = server.get(path).await.json();
        assert_eq!(doc["resource"], RESOURCE, "{path}");
        assert_eq!(doc["authorization_servers"], json!([BASE]));
        assert_eq!(doc["resource_name"], "Scotty");
        assert_eq!(doc["bearer_methods_supported"], json!(["header"]));
    }

    // The web login keeps its own /oauth/authorize (here without a provider
    // configured, so it says so instead of answering as grantry).
    let web = server.get("/oauth/authorize").await;
    assert!(
        web.text().contains("oauth_not_configured"),
        "{}",
        web.text()
    );
}

#[tokio::test]
async fn authorize_without_pkce_is_refused_to_the_client() {
    let (server, _state) = app().await;
    let client_id = client(&server).await;
    let response = authorize(
        &server,
        &authorize_query(&client_id, LOOPBACK, "read", false),
    )
    .await;
    let to = location(&response);
    assert!(to.starts_with(LOOPBACK), "{to}");
    assert_eq!(
        query_param(&to, "error").as_deref(),
        Some("invalid_request")
    );
    assert_eq!(query_param(&to, "state").as_deref(), Some("xyz"));
}

#[tokio::test]
async fn wrong_redirect_shows_an_error_page() {
    let (server, _state) = app().await;
    let client_id = client(&server).await;
    let response = authorize(
        &server,
        &authorize_query(&client_id, "https://evil.example/cb", "read", true),
    )
    .await;
    assert_eq!(response.status_code(), 400);
    assert!(response.headers().get(header::LOCATION).is_none());
    assert!(response.text().contains("not registered"));
}

#[tokio::test]
async fn loopback_redirect_with_another_port_is_accepted() {
    let (server, _state) = app().await;
    let client_id = client(&server).await;
    pending(
        &server,
        &client_id,
        "http://127.0.0.1:49152/callback",
        "read",
    )
    .await;
}

#[tokio::test]
async fn registration_is_rate_limited_per_address() {
    // Rate limiting is off in the config; registration is limited anyway,
    // with the oauth tier's numbers when that tier is set.
    let state = create_test_app_state_with_settings(
        settings(&[
            ("api.rate_limiting.enabled", "false"),
            ("api.rate_limiting.oauth.requests_per_minute", "2"),
            ("api.rate_limiting.oauth.burst_size", "2"),
        ]),
        None,
    )
    .await;
    let server = TestServer::new(ApiRoutes::create(state));
    for _ in 0..2 {
        assert_eq!(register(&server, "198.51.100.7").await.status_code(), 201);
    }
    let limited = register(&server, "198.51.100.7").await;
    assert_eq!(limited.status_code(), 429);
    assert!(limited.headers().contains_key(header::RETRY_AFTER));
    // Another address has its own budget.
    assert_eq!(register(&server, "198.51.100.8").await.status_code(), 201);
}

#[tokio::test]
async fn registration_without_an_address_shares_one_default_bucket() {
    // No tier configured: the default of burst 5.
    let (server, _state) = app().await;
    let anonymous = || async {
        server
            .post("/oauth/mcp/register")
            .json(&json!({ "redirect_uris": [LOOPBACK] }))
            .await
            .status_code()
            .as_u16()
    };
    for _ in 0..5 {
        assert_eq!(anonymous().await, 201);
    }
    assert_eq!(anonymous().await, 429);
}

// --- 4.2 Consent ------------------------------------------------------------------

#[tokio::test]
async fn approved_scopes_narrow_the_token_and_codes_are_single_use() {
    let (server, state) = app().await;
    let client_id = client(&server).await;
    let id = pending(&server, &client_id, LOOPBACK, "read create destroy").await;
    let alice = session(&state, "alice@example.com").await;

    let info = server
        .get(&format!("/api/v1/authenticated/oauth/consent/{id}"))
        .add_header(header::AUTHORIZATION, bearer(&alice))
        .await;
    assert_eq!(info.status_code(), 200, "{}", info.text());
    let info: Value = info.json();
    assert_eq!(info["id"], id.as_str());
    assert_eq!(info["client_id"], client_id.as_str());
    assert_eq!(info["client_name"], "Test Client");
    assert_eq!(info["redirect_host"], "127.0.0.1");
    assert_eq!(info["loopback_only"], false);
    assert_eq!(info["scopes"], json!(["read", "create", "destroy"]));

    // Alice's request now; bob does not see it, a service account is refused.
    let bob = session(&state, "bob@example.com").await;
    let other = server
        .get(&format!("/api/v1/authenticated/oauth/consent/{id}"))
        .add_header(header::AUTHORIZATION, bearer(&bob))
        .await;
    assert_eq!(other.status_code(), 404);
    let service = consent(
        &server,
        "test-bearer-token-123",
        &id,
        "approve",
        json!({"scopes": ["read"]}),
    )
    .await;
    assert_eq!(service.status_code(), 403);

    // Empty or wider scopes are refused and leave the request pending.
    assert_eq!(
        consent(&server, &alice, &id, "approve", json!({"scopes": []}))
            .await
            .status_code(),
        400
    );
    assert_eq!(
        consent(
            &server,
            &alice,
            &id,
            "approve",
            json!({"scopes": ["offline_access"]})
        )
        .await
        .status_code(),
        400
    );

    let approved = consent(&server, &alice, &id, "approve", json!({"scopes": ["read"]})).await;
    assert_eq!(approved.status_code(), 200, "{}", approved.text());
    let redirect = approved.json::<Value>()["redirect"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(redirect.starts_with(LOOPBACK), "{redirect}");
    assert_eq!(query_param(&redirect, "state").as_deref(), Some("xyz"));
    let code = query_param(&redirect, "code").unwrap();
    assert_eq!(
        consent(&server, &alice, &id, "approve", json!({"scopes": ["read"]}))
            .await
            .status_code(),
        404
    );

    let tokens = exchange(&server, &client_id, &code).await;
    assert_eq!(tokens.status_code(), 200, "{}", tokens.text());
    let tokens: Value = tokens.json();
    assert_eq!(tokens["scope"], "read");
    assert_eq!(tokens["expires_in"], 900);
    assert!(tokens.get("refresh_token").is_none());
    let claims = state
        .mcp_server
        .as_ref()
        .unwrap()
        .verify(tokens["access_token"].as_str().unwrap())
        .unwrap();
    assert_eq!(claims["scope"], "read");
    assert_eq!(claims["aud"], RESOURCE);

    // A second exchange fails, and revokes what the first one issued.
    let again = exchange(&server, &client_id, &code).await;
    assert_eq!(again.status_code(), 400);
    assert_eq!(again.json::<Value>()["error"], "invalid_grant");
    assert_challenge(&mcp(&server, tokens["access_token"].as_str()).await);
}

#[tokio::test]
async fn deny_redirects_with_access_denied() {
    let (server, state) = app().await;
    let client_id = client(&server).await;
    let id = pending(&server, &client_id, LOOPBACK, "read").await;
    let alice = session(&state, "alice@example.com").await;
    let denied = consent(&server, &alice, &id, "deny", json!({})).await;
    assert_eq!(denied.status_code(), 200, "{}", denied.text());
    let redirect = denied.json::<Value>()["redirect"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        query_param(&redirect, "error").as_deref(),
        Some("access_denied")
    );
    assert_eq!(
        consent(&server, &alice, &id, "deny", json!({}))
            .await
            .status_code(),
        404
    );
}

#[tokio::test]
async fn consent_page_is_served_by_the_frontend() {
    let (server, _state) = app().await;
    let page = server
        .get(&format!("/oauth/consent/{}", uuid::Uuid::new_v4()))
        .await;
    let spa = server.get("/no-such-backend-route").await;
    assert_eq!(page.status_code(), spa.status_code());
    assert_eq!(page.text(), spa.text());
}

// --- 4.3 /mcp ---------------------------------------------------------------------

#[tokio::test]
async fn mcp_requires_an_active_mcp_token() {
    let (server, state) = app().await;
    let (_client, tokens) = connect(&server, &state, "alice@example.com", "read", &["read"]).await;
    let access = tokens["access_token"].as_str().unwrap();

    let response = mcp(&server, None).await;
    assert_challenge(&response);
    assert!(!response.headers()[header::WWW_AUTHENTICATE]
        .to_str()
        .unwrap()
        .contains("invalid_token"));
    assert_challenge(&mcp(&server, Some("garbage")).await);
    let alice_session = session(&state, "alice@example.com").await;
    assert_challenge(&mcp(&server, Some(&alice_session)).await);
    // An mcp token is no API session either.
    let api = server
        .post("/api/v1/authenticated/validate-token")
        .add_header(header::AUTHORIZATION, bearer(access))
        .await;
    assert_eq!(api.status_code(), 401);

    assert_eq!(mcp(&server, Some(access)).await.status_code(), 200);

    let server_ = state.mcp_server.as_ref().unwrap();
    let (grant, _) = server_.verify_active(access).await.unwrap();
    server_.revoke(&grant, "test", None).await.unwrap();
    assert_challenge(&mcp(&server, Some(access)).await);
}

#[tokio::test]
async fn allow_needs_both_the_scope_and_the_permission() {
    let (_server, state) = app().await;
    let default = ["default".to_string()];
    let principal = |email: &str, scopes: &[&str]| McpPrincipal {
        email: email.into(),
        scopes: scopes.iter().map(|s| s.to_string()).collect(),
    };
    let target = McpTarget::Scopes(&default);

    // The admin may destroy, but the token lacks `destroy`.
    let admin_read = principal(ADMIN, &["read"]);
    assert!(
        admin_read
            .allow(&state, McpScope::Read, Permission::View, target)
            .await
    );
    assert!(
        !admin_read
            .allow(&state, McpScope::Destroy, Permission::Destroy, target)
            .await
    );
    let admin_destroy = principal(ADMIN, &["read", "destroy"]);
    assert!(
        admin_destroy
            .allow(&state, McpScope::Destroy, Permission::Destroy, target)
            .await
    );

    // Alice (viewer) holds `destroy` in the token but not in the policy.
    let alice = principal("alice@example.com", &["read", "create", "destroy"]);
    assert!(
        alice
            .allow(&state, McpScope::Read, Permission::View, target)
            .await
    );
    assert!(
        !alice
            .allow(&state, McpScope::Destroy, Permission::Destroy, target)
            .await
    );
    assert!(
        !alice
            .allow(&state, McpScope::Create, Permission::Create, target)
            .await
    );

    // A scope never stands for a permission outside its mapping.
    assert!(
        !admin_destroy
            .allow(&state, McpScope::Read, Permission::Shell, target)
            .await
    );
    assert!(
        !admin_destroy
            .allow(&state, McpScope::Destroy, Permission::Manage, target)
            .await
    );
}

// --- 4.4 Connections ---------------------------------------------------------------

async fn connections(server: &TestServer, token: &str, all: bool) -> TestResponse {
    let mut request = server
        .get("/api/v1/authenticated/connections")
        .add_header(header::AUTHORIZATION, bearer(token));
    if all {
        request = request.add_query_param("all", "true");
    }
    request.await
}

async fn revoke(server: &TestServer, token: &str, id: &str) -> u16 {
    server
        .delete(&format!("/api/v1/authenticated/connections/{id}"))
        .add_header(header::AUTHORIZATION, bearer(token))
        .await
        .status_code()
        .as_u16()
}

#[tokio::test]
async fn connections_list_own_or_all_for_admins() {
    let (server, state) = app().await;
    connect(
        &server,
        &state,
        "alice@example.com",
        "read offline_access",
        &["read", "offline_access"],
    )
    .await;
    state
        .session_server
        .as_ref()
        .unwrap()
        .issue_grant(
            "alice@example.com".to_string(),
            "scottyctl",
            "scottyctl",
            "api offline_access",
        )
        .await
        .unwrap();
    connect(&server, &state, "bob@example.com", "read", &["read"]).await;
    let alice = session(&state, "alice@example.com").await;

    let own = connections(&server, &alice, false).await;
    assert_eq!(own.status_code(), 200, "{}", own.text());
    let own: Vec<Value> = own.json();
    // The browser session (scotty-web) is not listed.
    assert_eq!(own.len(), 2, "{own:#?}");
    let mcp_row = own.iter().find(|c| c["kind"] == "mcp").unwrap();
    assert_eq!(mcp_row["client_name"], "Test Client");
    assert_eq!(mcp_row["redirect_host"], "127.0.0.1");
    assert_eq!(mcp_row["scope"], "read offline_access");
    assert_eq!(mcp_row["subject"], "alice@example.com");
    assert!(mcp_row["last_used_at"].is_null());
    assert!(chrono::DateTime::parse_from_rfc3339(mcp_row["created_at"].as_str().unwrap()).is_ok());
    assert!(chrono::DateTime::parse_from_rfc3339(mcp_row["expires_at"].as_str().unwrap()).is_ok());
    let cli_row = own.iter().find(|c| c["kind"] == "cli").unwrap();
    assert_eq!(cli_row["client_id"], "scottyctl");
    assert!(cli_row["redirect_host"].is_null());

    assert_eq!(connections(&server, &alice, true).await.status_code(), 403);
    assert_eq!(
        connections(&server, "test-bearer-token-123", false)
            .await
            .status_code(),
        403
    );

    let admin = session(&state, ADMIN).await;
    let all: Vec<Value> = connections(&server, &admin, true).await.json();
    assert_eq!(all.len(), 3, "{all:#?}");
    assert!(connections(&server, &admin, false)
        .await
        .json::<Vec<Value>>()
        .is_empty());
}

#[tokio::test]
async fn revoking_a_connection_stops_refresh_and_mcp() {
    let (server, state) = app().await;
    let (client_id, tokens) = connect(
        &server,
        &state,
        "alice@example.com",
        "read offline_access",
        &["read", "offline_access"],
    )
    .await;
    let access = tokens["access_token"].as_str().unwrap();
    let refresh_token = tokens["refresh_token"].as_str().unwrap();
    let alice = session(&state, "alice@example.com").await;
    let bob = session(&state, "bob@example.com").await;
    let id = connections(&server, &alice, false)
        .await
        .json::<Vec<Value>>()[0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Not bob's to see or revoke; unknown ids are 404 too.
    assert_eq!(revoke(&server, &bob, &id).await, 404);
    assert_eq!(
        revoke(&server, &alice, &uuid::Uuid::new_v4().to_string()).await,
        404
    );
    assert_eq!(revoke(&server, "test-bearer-token-123", &id).await, 403);
    assert_eq!(mcp(&server, Some(access)).await.status_code(), 200);

    assert_eq!(revoke(&server, &alice, &id).await, 204);
    assert_challenge(&mcp(&server, Some(access)).await);
    let refreshed = refresh(&server, &client_id, refresh_token).await;
    assert_eq!(refreshed.status_code(), 400);
    assert_eq!(refreshed.json::<Value>()["error"], "invalid_grant");
    assert_eq!(revoke(&server, &alice, &id).await, 404);
    assert!(connections(&server, &alice, false)
        .await
        .json::<Vec<Value>>()
        .is_empty());
}

#[tokio::test]
async fn admins_revoke_any_connection_and_refresh_works_until_then() {
    let (server, state) = app().await;
    let (client_id, tokens) = connect(
        &server,
        &state,
        "bob@example.com",
        "read offline_access",
        &["read", "offline_access"],
    )
    .await;
    let refreshed = refresh(
        &server,
        &client_id,
        tokens["refresh_token"].as_str().unwrap(),
    )
    .await;
    assert_eq!(refreshed.status_code(), 200, "{}", refreshed.text());
    let refreshed: Value = refreshed.json();

    let admin = session(&state, ADMIN).await;
    let all: Vec<Value> = connections(&server, &admin, true).await.json();
    assert!(!all[0]["last_used_at"].is_null());
    let id = all[0]["id"].as_str().unwrap();
    assert_eq!(revoke(&server, &admin, id).await, 204);
    let after = refresh(
        &server,
        &client_id,
        refreshed["refresh_token"].as_str().unwrap(),
    )
    .await;
    assert_eq!(after.status_code(), 400);
}
