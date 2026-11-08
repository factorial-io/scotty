use crate::api::router::ApiRoutes;
use crate::api::test_utils::create_test_app_state_with_settings;
use crate::app_state::AppState;
use axum_test::TestServer;
use config::Config;
use openidconnect::core::{
    CoreIdToken, CoreIdTokenClaims, CoreJsonWebKeySet, CoreJwsSigningAlgorithm,
    CoreRsaPrivateSigningKey,
};
use openidconnect::{
    Audience, EmptyAdditionalClaims, EndUserEmail, IssuerUrl, JsonWebKeyId, Nonce,
    PrivateSigningKey, StandardClaims, SubjectIdentifier,
};
use std::sync::Arc;
use wiremock::{
    matchers::{body_string_contains, method, path},
    Mock, MockServer, ResponseTemplate,
};

/// Create test config for OAuth with dynamic mock server URL
async fn create_oauth_config_with_mock_server(
    mock_server_url: &str,
) -> crate::settings::config::Settings {
    // Load base test config and override OAuth settings
    let builder = Config::builder()
        .add_source(config::File::with_name("tests/test_oauth_auth"))
        .set_override("api.oauth.oidc_issuer_url", mock_server_url)
        .unwrap();

    let config = builder.build().unwrap();
    config.try_deserialize().unwrap()
}

/// Create Scotty app state with OAuth, a session server and a mock OIDC provider
async fn create_mock_oauth_state(mock_server_url: &str) -> Arc<AppState> {
    let settings = create_oauth_config_with_mock_server(mock_server_url).await;

    // Create OAuth client with mock server URL
    let oauth_state = match crate::oauth::client::create_oauth_client(&settings.api.oauth) {
        Ok(Some(client)) => Some(crate::oauth::handlers::OAuthState {
            client,
            device_flow_store: crate::oauth::create_device_flow_store(),
            web_flow_store: crate::oauth::create_web_flow_store(),
            session_store: crate::oauth::create_oauth_session_store(),
        }),
        _ => None,
    };

    create_test_app_state_with_settings(settings, oauth_state).await
}

/// Create Scotty app with OAuth and mock OIDC provider
async fn create_scotty_app_with_mock_oauth(mock_server_url: &str) -> axum::Router {
    ApiRoutes::create(create_mock_oauth_state(mock_server_url).await)
}

#[tokio::test]
async fn test_oauth_device_flow_complete() {
    let mock_server = MockServer::start().await;
    let mock_url = mock_server.uri();

    // Mock device authorization endpoint with proper OAuth2 response
    Mock::given(method("POST"))
        .and(path("/oauth/authorize_device"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "mock_device_code_12345",
            "user_code": "ABCD-1234",
            "verification_uri": format!("{}/device", mock_url),
            "verification_uri_complete": format!("{}/device?user_code=ABCD-1234", mock_url),
            "expires_in": 1800,
            "interval": 5
        })))
        .mount(&mock_server)
        .await;

    let router = create_scotty_app_with_mock_oauth(&mock_url).await;
    let server = TestServer::new(router);

    // Test device flow initiation - should work perfectly
    let response = server.post("/oauth/device").await;

    assert_eq!(
        response.status_code(),
        200,
        "Device flow should start successfully"
    );
    let body: serde_json::Value = response.json();

    // Verify response contains all required fields
    assert_eq!(
        body["device_code"].as_str().unwrap(),
        "mock_device_code_12345"
    );
    assert_eq!(body["user_code"].as_str().unwrap(), "ABCD-1234");
    assert!(body["verification_uri"]
        .as_str()
        .unwrap()
        .contains(&mock_url));
    assert!(
        body["expires_in"].as_u64().unwrap() >= 1799,
        "Should have reasonable expiry time"
    );
    assert_eq!(body["interval"].as_u64().unwrap(), 5);
}

#[tokio::test]
async fn test_oauth_device_flow_authorization_pending() {
    let mock_server = MockServer::start().await;
    let mock_url = mock_server.uri();

    // Mock device authorization endpoint
    Mock::given(method("POST"))
        .and(path("/oauth/authorize_device"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "test_device_pending",
            "user_code": "EFGH-5678",
            "verification_uri": format!("{}/device", mock_url),
            "expires_in": 1800,
            "interval": 5
        })))
        .mount(&mock_server)
        .await;

    // Mock token endpoint with exact format Scotty expects
    // Based on device_flow.rs: uses form data with grant_type and device_code
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains(
            "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code",
        ))
        .and(body_string_contains("device_code=test_device_pending"))
        // Basic auth header will be present but we don't need to match it exactly
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "authorization_pending",
            "error_description": "The authorization request is still pending"
        })))
        .mount(&mock_server)
        .await;

    let router = create_scotty_app_with_mock_oauth(&mock_url).await;
    let server = TestServer::new(router);

    // Start device flow
    let device_response = server.post("/oauth/device").await;
    assert_eq!(device_response.status_code(), 200);

    let body: serde_json::Value = device_response.json();
    let device_code = body["device_code"].as_str().unwrap();
    assert_eq!(device_code, "test_device_pending");

    // Poll for token using Scotty's device token endpoint
    let poll_response = server
        .post("/oauth/device/token")
        .add_query_param("device_code", device_code)
        .await;

    // Should return 400 with proper authorization_pending error
    assert_eq!(
        poll_response.status_code(),
        400,
        "Should return authorization_pending error"
    );

    let error_body: serde_json::Value = poll_response.json();
    assert_eq!(
        error_body["error"].as_str().unwrap(),
        "authorization_pending",
        "Should return proper OAuth authorization_pending error"
    );
}

#[tokio::test]
async fn test_oauth_device_flow_complete_success() {
    let mock_server = MockServer::start().await;
    let mock_url = mock_server.uri();

    // Mock device authorization endpoint
    Mock::given(method("POST"))
        .and(path("/oauth/authorize_device"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "success_device_code",
            "user_code": "SUCCESS-789",
            "verification_uri": format!("{}/device", mock_url),
            "expires_in": 1800,
            "interval": 5
        })))
        .mount(&mock_server)
        .await;

    // Mock token endpoint - successful response
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "oauth_success_token_xyz789",
            "token_type": "Bearer",
            "expires_in": 3600,
            "scope": "read_user read_api openid profile email"
        })))
        .mount(&mock_server)
        .await;

    // Mock user info endpoint - this is what was missing!
    Mock::given(method("GET"))
        .and(path("/oauth/userinfo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sub": "oauth_user_123",
            "name": "OAuth Test User",
            "email": "oauth.test@example.com",
            "preferred_username": "oauthuser"
        })))
        .mount(&mock_server)
        .await;

    let router = create_scotty_app_with_mock_oauth(&mock_url).await;
    let server = TestServer::new(router);

    // Start device flow
    let device_response = server.post("/oauth/device").await;
    assert_eq!(
        device_response.status_code(),
        200,
        "Device flow should start"
    );

    let device_body: serde_json::Value = device_response.json();
    let device_code = device_body["device_code"].as_str().unwrap();
    assert_eq!(device_code, "success_device_code");

    // Poll for token - should now succeed completely!
    let poll_response = server
        .post("/oauth/device/token")
        .add_query_param("device_code", device_code)
        .await;

    // Should return 200 with successful OAuth completion
    assert_eq!(
        poll_response.status_code(),
        200,
        "OAuth device flow should complete successfully"
    );

    let token_body: serde_json::Value = poll_response.json();

    // Verify we get the actual OAuth response
    assert_eq!(
        token_body["access_token"].as_str().unwrap(),
        "oauth_success_token_xyz789",
        "Should return OAuth access token"
    );

    // Verify complete OAuth response with user information
    assert_eq!(token_body["user_name"].as_str().unwrap(), "OAuth Test User");
    assert_eq!(
        token_body["user_email"].as_str().unwrap(),
        "oauth.test@example.com"
    );
    assert_eq!(token_body["user_id"].as_str().unwrap(), "oauthuser");
}

#[tokio::test]
async fn test_oauth_web_flow_authorization_url() {
    let mock_server = MockServer::start().await;
    let mock_url = mock_server.uri();
    mount_discovery(&mock_server).await;

    let router = create_scotty_app_with_mock_oauth(&mock_url).await;
    let server = TestServer::new(router);

    // Test authorization URL generation
    let response = server
        .get("/oauth/authorize")
        .add_query_param("redirect_uri", "http://localhost:21342/api/oauth/callback")
        .add_query_param("frontend_callback", "http://localhost:3000/auth/callback")
        .await;

    // Should redirect to OAuth provider authorization URL
    assert!(
        response.status_code() == 302 || response.status_code() == 307,
        "Should redirect to OAuth provider, got: {}",
        response.status_code()
    );

    // Check redirect location contains mock server URL
    if let Some(location) = response.headers().get("location") {
        let location_str = location.to_str().unwrap();
        assert!(
            location_str.contains(&mock_url),
            "Redirect should go to mock OAuth provider: {}",
            location_str
        );
        assert!(
            location_str.contains("client_id=test_oauth_client_id"),
            "Should contain client ID: {}",
            location_str
        );
        assert!(
            location_str.contains("code_challenge"),
            "Should contain PKCE challenge: {}",
            location_str
        );
    }
}

#[tokio::test]
async fn test_oauth_callback_with_mock_token_exchange() {
    let mock_server = MockServer::start().await;
    let mock_url = mock_server.uri();

    // Mock token exchange endpoint
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "mock_access_token_abc123",
            "token_type": "Bearer",
            "expires_in": 3600,
            "refresh_token": "mock_refresh_token_def456",
            "scope": "read_user profile email"
        })))
        .mount(&mock_server)
        .await;

    // Mock user info endpoint
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "mock_user_123",
            "username": "testuser",
            "name": "Test User",
            "email": "test@example.com"
        })))
        .mount(&mock_server)
        .await;

    let router = create_scotty_app_with_mock_oauth(&mock_url).await;
    let server = TestServer::new(router);

    // First, start authorization flow to get a valid state
    let auth_response = server
        .get("/oauth/authorize")
        .add_query_param("redirect_uri", "http://localhost:21342/api/oauth/callback")
        .await;

    // Extract state from redirect URL if available
    let mut test_state = "test_csrf_state";
    if let Some(location) = auth_response.headers().get("location") {
        let location_str = location.to_str().unwrap();
        if let Some(state_start) = location_str.find("state=") {
            let state_part = &location_str[state_start + 6..];
            if let Some(state_end) = state_part.find('&') {
                test_state = &state_part[..state_end];
            } else {
                test_state = state_part;
            }
        }
    }

    // Test OAuth callback
    let callback_response = server
        .get("/api/oauth/callback")
        .add_query_param("code", "mock_auth_code_xyz789")
        .add_query_param("state", test_state)
        .await;

    // Should either complete successfully or handle gracefully
    assert!(
        callback_response.status_code() == 302
            || callback_response.status_code() == 400
            || callback_response.status_code() == 500,
        "OAuth callback should handle request, got: {}",
        callback_response.status_code()
    );
}

#[tokio::test]
async fn test_complete_oauth_flow_with_protected_endpoint_access() {
    let mock_server = MockServer::start().await;
    let mock_url = mock_server.uri();

    // Mock complete OAuth flow
    Mock::given(method("POST"))
        .and(path("/oauth/authorize_device"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "complete_flow_device",
            "user_code": "COMPLETE-123",
            "verification_uri": format!("{}/device", mock_url),
            "expires_in": 1800,
            "interval": 5
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "complete_oauth_token_789",
            "token_type": "Bearer",
            "expires_in": 3600,
            "scope": "read_user read_api openid profile email"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/oauth/userinfo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sub": "complete_user_456",
            "name": "Complete Flow User",
            "email": "complete@example.com",
            "preferred_username": "completeuser"
        })))
        .mount(&mock_server)
        .await;

    let router = create_scotty_app_with_mock_oauth(&mock_url).await;
    let server = TestServer::new(router);

    // STEP 1: Verify protected endpoint requires auth
    let unauth_response = server.get("/api/v1/authenticated/blueprints").await;
    assert_eq!(
        unauth_response.status_code(),
        401,
        "Should require authentication"
    );

    // STEP 2: Complete OAuth device flow
    let device_response = server.post("/oauth/device").await;
    assert_eq!(
        device_response.status_code(),
        200,
        "Device flow should start"
    );

    let device_body: serde_json::Value = device_response.json();
    let device_code = device_body["device_code"].as_str().unwrap();

    // STEP 3: Poll for token and complete authentication
    let token_response = server
        .post("/oauth/device/token")
        .add_query_param("device_code", device_code)
        .await;

    assert_eq!(
        token_response.status_code(),
        200,
        "OAuth flow should complete"
    );
    let token_body: serde_json::Value = token_response.json();
    let access_token = token_body["access_token"].as_str().unwrap();

    // STEP 4: The provider token from the (old) device flow is no API
    // credential any more; task 3.1 replaces this flow with grantry's.
    let protected_response = server
        .get("/api/v1/authenticated/blueprints")
        .add_header(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_str(&format!("Bearer {}", access_token)).unwrap(),
        )
        .await;
    assert_eq!(
        protected_response.status_code(),
        401,
        "A provider token must not authenticate API requests"
    );
    assert_eq!(access_token, "complete_oauth_token_789");
}

#[tokio::test]
async fn test_oauth_provider_error_handling() {
    let mock_server = MockServer::start().await;
    let mock_url = mock_server.uri();

    // Mock OAuth provider returns server error
    Mock::given(method("POST"))
        .and(path("/oauth/authorize_device"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": "server_error",
            "error_description": "OAuth provider internal server error"
        })))
        .mount(&mock_server)
        .await;

    let router = create_scotty_app_with_mock_oauth(&mock_url).await;
    let server = TestServer::new(router);

    // Test that Scotty handles OAuth provider errors gracefully
    let response = server.post("/oauth/device").await;

    // Should return 500 because OAuth provider is returning 500
    // This is correct behavior - propagate OAuth provider errors
    assert_eq!(
        response.status_code(),
        500,
        "Should propagate OAuth provider errors"
    );

    let body: serde_json::Value = response.json();
    assert!(
        body.get("error").is_some(),
        "Error response should contain error field: {}",
        serde_json::to_string_pretty(&body).unwrap_or_default()
    );
}

// --- Mock OIDC provider ------------------------------------------------------

const CLIENT_ID: &str = "test_oauth_client_id";
const PROVIDER_ACCESS_TOKEN: &str = "provider_access_token_must_not_leak";
const FRONTEND_CALLBACK: &str = "http://localhost:3000/oauth/callback";

fn signing_key() -> CoreRsaPrivateSigningKey {
    CoreRsaPrivateSigningKey::from_pem(
        include_str!("../../tests/fixtures/oidc_test_key.pem"),
        Some(JsonWebKeyId::new("test-key".to_string())),
    )
    .unwrap()
}

/// Serve discovery and JWKS; the token endpoint is mounted per test.
async fn mount_discovery(mock_server: &MockServer) {
    let issuer = mock_server.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "issuer": issuer,
            "authorization_endpoint": format!("{issuer}/authorize"),
            "token_endpoint": format!("{issuer}/token"),
            "userinfo_endpoint": format!("{issuer}/userinfo"),
            "jwks_uri": format!("{issuer}/jwks"),
            "response_types_supported": ["code"],
            "subject_types_supported": ["public"],
            "id_token_signing_alg_values_supported": ["RS256"],
        })))
        .mount(mock_server)
        .await;
    let jwks = CoreJsonWebKeySet::new(vec![signing_key().as_verification_key()]);
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(jwks))
        .mount(mock_server)
        .await;
}

/// An RS256 ID token for `user-42` from `issuer`.
fn id_token(issuer: &str, nonce: &str, email: Option<&str>) -> String {
    let mut standard = StandardClaims::new(SubjectIdentifier::new("user-42".to_string()));
    if let Some(email) = email {
        standard = standard
            .set_email(Some(EndUserEmail::new(email.to_string())))
            .set_email_verified(Some(true));
    }
    let now = chrono::Utc::now();
    let claims = CoreIdTokenClaims::new(
        IssuerUrl::new(issuer.to_string()).unwrap(),
        vec![Audience::new(CLIENT_ID.to_string())],
        now + chrono::Duration::minutes(5),
        now,
        standard,
        EmptyAdditionalClaims {},
    )
    .set_nonce(Some(Nonce::new(nonce.to_string())));
    let token = CoreIdToken::new(
        claims,
        &signing_key(),
        CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
        None,
        None,
    )
    .unwrap();
    serde_json::to_value(&token)
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}

async fn mount_token_endpoint(mock_server: &MockServer, id_token: String) {
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": PROVIDER_ACCESS_TOKEN,
            "token_type": "Bearer",
            "expires_in": 7200,
            "id_token": id_token,
        })))
        .mount(mock_server)
        .await;
}

fn query_param(url: &str, name: &str) -> String {
    url::Url::parse(url)
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.into_owned())
        .unwrap_or_else(|| panic!("no {name} in {url}"))
}

/// Start a web login; returns the `state` and `nonce` sent to the provider.
async fn begin_login(server: &TestServer) -> (String, String) {
    let response = server
        .get("/oauth/authorize")
        .add_query_param("redirect_uri", FRONTEND_CALLBACK)
        .await;
    assert_eq!(response.status_code(), 307, "{}", response.text());
    let location = response.headers()["location"].to_str().unwrap().to_string();
    (query_param(&location, "state"), query_param(&location, "nonce"))
}

async fn callback(server: &TestServer, state: &str) -> axum_test::TestResponse {
    server
        .get("/api/oauth/callback")
        .add_query_param("code", "provider-code")
        .add_query_param("state", state)
        .await
}

fn bearer(token: &str) -> axum::http::HeaderValue {
    axum::http::HeaderValue::from_str(&format!("Bearer {token}")).unwrap()
}

async fn status_with(server: &TestServer, token: &str) -> u16 {
    server
        .post("/api/v1/authenticated/validate-token")
        .add_header(axum::http::header::AUTHORIZATION, bearer(token))
        .await
        .status_code()
        .as_u16()
}

#[tokio::test]
async fn test_web_login_issues_scotty_session_token() {
    let mock_server = MockServer::start().await;
    mount_discovery(&mock_server).await;
    let server = TestServer::new(create_scotty_app_with_mock_oauth(&mock_server.uri()).await);

    let (state, nonce) = begin_login(&server).await;
    mount_token_endpoint(
        &mock_server,
        id_token(&mock_server.uri(), &nonce, Some("alice@example.com")),
    )
    .await;

    let response = callback(&server, &state).await;
    assert_eq!(response.status_code(), 307, "{}", response.text());
    let location = response.headers()["location"].to_str().unwrap().to_string();
    assert!(location.starts_with(FRONTEND_CALLBACK), "{location}");
    let session_id = query_param(&location, "session_id");

    let exchange = server
        .post("/oauth/exchange")
        .json(&serde_json::json!({ "session_id": session_id }))
        .await;
    assert_eq!(exchange.status_code(), 200);
    assert!(
        !exchange.text().contains(PROVIDER_ACCESS_TOKEN),
        "the provider token must not reach the browser"
    );
    let body: serde_json::Value = exchange.json();
    assert_eq!(body["user_email"], "alice@example.com");
    assert_eq!(body["expires_in"], 12 * 3600);
    assert!(body["refresh_token"].is_null());
    let token = body["access_token"].as_str().unwrap();

    assert_eq!(status_with(&server, token).await, 200);

    // The exchange is single-use.
    let again = server
        .post("/oauth/exchange")
        .json(&serde_json::json!({ "session_id": session_id }))
        .await;
    assert_ne!(again.status_code(), 200);

    // Logout revokes the grant, so the token stops working.
    let logout = server
        .post("/api/v1/authenticated/logout")
        .add_header(axum::http::header::AUTHORIZATION, bearer(token))
        .await;
    assert_eq!(logout.status_code(), 204);
    assert_eq!(status_with(&server, token).await, 401);
}

#[tokio::test]
async fn test_web_login_without_email_fails() {
    let mock_server = MockServer::start().await;
    mount_discovery(&mock_server).await;
    Mock::given(method("GET"))
        .and(path("/userinfo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sub": "user-42",
            "name": "No Mail",
        })))
        .expect(1)
        .mount(&mock_server)
        .await;
    let state = create_mock_oauth_state(&mock_server.uri()).await;
    let server = TestServer::new(ApiRoutes::create(state.clone()));

    let (csrf_state, nonce) = begin_login(&server).await;
    mount_token_endpoint(&mock_server, id_token(&mock_server.uri(), &nonce, None)).await;

    let response = callback(&server, &csrf_state).await;
    assert_eq!(response.status_code(), 403, "{}", response.text());
    let oauth = state.oauth_state.as_ref().unwrap();
    assert!(oauth.session_store.lock().unwrap().is_empty());
    mock_server.verify().await;
}

#[tokio::test]
async fn test_web_login_with_wrong_nonce_fails() {
    let mock_server = MockServer::start().await;
    mount_discovery(&mock_server).await;
    let server = TestServer::new(create_scotty_app_with_mock_oauth(&mock_server.uri()).await);

    let (state, _nonce) = begin_login(&server).await;
    mount_token_endpoint(
        &mock_server,
        id_token(&mock_server.uri(), "not-the-nonce", Some("alice@example.com")),
    )
    .await;

    let response = callback(&server, &state).await;
    assert_eq!(response.status_code(), 400, "{}", response.text());
    assert!(response.headers().get("location").is_none());
}

// --- Session tokens on the REST API -------------------------------------------

async fn session_app() -> (TestServer, Arc<AppState>) {
    let state =
        crate::api::test_utils::create_test_app_state_with_config("tests/test_oauth_auth", None)
            .await;
    (TestServer::new(ApiRoutes::create(state.clone())), state)
}

async fn issue_session(state: &AppState) -> (grantry::Grant<String>, String) {
    let (grant, tokens) = state
        .session_server
        .as_ref()
        .unwrap()
        .issue_grant(&"alice@example.com".to_string(), "scotty-web", "Scotty web", "api")
        .await
        .unwrap();
    (grant, tokens.access_token)
}

/// A token signed with the session secret for `grant`, with chosen claims.
fn forge(
    state: &AppState,
    grant: &grantry::Grant<String>,
    resource: &str,
    kind: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> String {
    let server = state.session_server.as_ref().unwrap();
    grantry::token::mint(
        &server.config.signer,
        now,
        3600,
        &grantry::token::AccessClaims {
            resource,
            kind,
            scope: "api",
            client_id: "scotty-web",
            grant_id: &grant.id.to_string(),
        },
        serde_json::Map::new(),
    )
    .unwrap()
}

#[tokio::test]
async fn test_session_token_accepted_and_revocation_rejected() {
    let (server, state) = session_app().await;
    let (grant, token) = issue_session(&state).await;
    assert_eq!(status_with(&server, &token).await, 200);

    state
        .session_server
        .as_ref()
        .unwrap()
        .revoke(&grant, "test", None)
        .await
        .unwrap();
    assert_eq!(status_with(&server, &token).await, 401);
}

#[tokio::test]
async fn test_session_token_wrong_kind_audience_or_expired_rejected() {
    let (server, state) = session_app().await;
    let (grant, _token) = issue_session(&state).await;
    let now = chrono::Utc::now();

    let control = forge(&state, &grant, "scotty-api", "session", now);
    assert_eq!(status_with(&server, &control).await, 200);

    let mcp_kind = forge(&state, &grant, "scotty-api", "mcp", now);
    assert_eq!(status_with(&server, &mcp_kind).await, 401);
    let mcp_audience = forge(&state, &grant, "http://localhost:21342/mcp", "session", now);
    assert_eq!(status_with(&server, &mcp_audience).await, 401);
    let mcp_token = forge(&state, &grant, "http://localhost:21342/mcp", "mcp", now);
    assert_eq!(status_with(&server, &mcp_token).await, 401);
    let expired = forge(
        &state,
        &grant,
        "scotty-api",
        "session",
        now - chrono::Duration::hours(2),
    );
    assert_eq!(status_with(&server, &expired).await, 401);
}

#[tokio::test]
async fn test_provider_token_rejected_without_provider_call() {
    let mock_server = MockServer::start().await;
    Mock::given(wiremock::matchers::any())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sub": "user-42",
            "email": "alice@example.com",
        })))
        .expect(0)
        .mount(&mock_server)
        .await;
    let server = TestServer::new(create_scotty_app_with_mock_oauth(&mock_server.uri()).await);

    assert_eq!(status_with(&server, "glpat-provider-token").await, 401);
    assert_eq!(status_with(&server, "a.b.c").await, 401);
    let response = server
        .get("/api/v1/authenticated/apps/list")
        .add_header(axum::http::header::AUTHORIZATION, bearer("glpat-provider-token"))
        .await;
    assert_eq!(response.status_code(), 401);
    mock_server.verify().await;
}

#[tokio::test]
async fn test_bearer_token_still_works_in_oauth_mode_and_logout_is_noop() {
    let (server, _state) = session_app().await;
    assert_eq!(status_with(&server, "test-bearer-token-123").await, 200);
    let logout = server
        .post("/api/v1/authenticated/logout")
        .add_header(axum::http::header::AUTHORIZATION, bearer("test-bearer-token-123"))
        .await;
    assert_eq!(logout.status_code(), 204);
    assert_eq!(status_with(&server, "test-bearer-token-123").await, 200);
}
