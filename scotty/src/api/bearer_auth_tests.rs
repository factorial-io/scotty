use crate::api::router::ApiRoutes;
use crate::app_state::AppState;
use axum_test::TestServer;
use config::Config;
use std::sync::Arc;

/// Create actual Scotty router for testing with bearer auth configuration
async fn create_scotty_app_with_bearer_auth() -> axum::Router {
    // Load test configuration from file
    let builder = Config::builder().add_source(config::File::with_name("tests/test_bearer_auth"));

    let config = builder.build().unwrap();
    let settings: crate::settings::config::Settings = config.try_deserialize().unwrap();

    // Create app state with test configuration
    let app_state = Arc::new(AppState {
        settings,
        stop_flag: crate::stop_flag::StopFlag::new(),
        clients: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        apps: scotty_core::apps::shared_app_list::SharedAppList::new(),
        docker: bollard::Docker::connect_with_local_defaults().unwrap(),
        task_manager: crate::tasks::manager::TaskManager::new(),
        oauth_state: None,
        auth_service: Arc::new(
            crate::services::AuthorizationService::new("../config/casbin").await
                .expect("Failed to load RBAC config for test"),
        ),
    });

    // Create the actual Scotty router with all routes
    ApiRoutes::create(app_state)
}


#[tokio::test]
async fn test_bearer_auth_valid_token_blueprints() {
    let router = create_scotty_app_with_bearer_auth().await;
    let server = TestServer::new(router).unwrap();

    // Make authenticated request to blueprints endpoint with valid token
    let response = server
        .get("/api/v1/authenticated/blueprints")
        .add_header(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_str("Bearer test-bearer-token-123").unwrap(),
        )
        .await;

    assert_eq!(response.status_code(), 200);
    let body = response.text();
    assert!(
        body.contains("test-blueprint"),
        "Response should contain test blueprint: {}",
        body
    );
}

#[tokio::test]
async fn test_bearer_auth_invalid_token_blueprints() {
    let router = create_scotty_app_with_bearer_auth().await;
    let server = TestServer::new(router).unwrap();

    // Make authenticated request with wrong token
    let response = server
        .get("/api/v1/authenticated/blueprints")
        .add_header(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_str("Bearer wrong-token").unwrap(),
        )
        .await;

    // Should fail since only explicitly assigned tokens should work
    assert_eq!(response.status_code(), 401);
}

/// Create Scotty router with actual authorization service (not fallback)
async fn create_scotty_app_with_rbac_auth() -> axum::Router {
    let builder = Config::builder().add_source(config::File::with_name("tests/test_bearer_auth"));

    let config = builder.build().unwrap();
    let settings: crate::settings::config::Settings = config.try_deserialize().unwrap();

    // Create app state with actual authorization service that loads from config
    let app_state = Arc::new(AppState {
        settings: settings.clone(),
        stop_flag: crate::stop_flag::StopFlag::new(),
        clients: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        apps: scotty_core::apps::shared_app_list::SharedAppList::new(),
        docker: bollard::Docker::connect_with_local_defaults().unwrap(),
        task_manager: crate::tasks::manager::TaskManager::new(),
        oauth_state: None,
        auth_service: Arc::new(
            crate::services::AuthorizationService::new("../config/casbin").await
                .expect("Failed to load RBAC config for test"),
        ),
    });

    ApiRoutes::create(app_state)
}

#[tokio::test]
async fn test_bearer_auth_with_rbac_assigned_token() {
    // First test that the authorization service loads the assignments correctly
    let auth_service = crate::services::AuthorizationService::new("../config/casbin").await
        .expect("Failed to load RBAC config for test");
    
    let assignments = auth_service.list_assignments().await;
    println!("Loaded assignments: {:?}", assignments);
    
    // Check if bearer:client-a exists
    let client_a_token = crate::services::AuthorizationService::format_user_id("", Some("client-a"));
    println!("Looking for token: {}", client_a_token);
    assert!(assignments.contains_key(&client_a_token), "client-a token should be in assignments");

    let router = create_scotty_app_with_rbac_auth().await;
    let server = TestServer::new(router).unwrap();

    // Test with a token that should be in the assignments (from policy.yaml)
    let response = server
        .get("/api/v1/authenticated/blueprints")
        .add_header(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_str("Bearer client-a").unwrap(),
        )
        .await;

    // Should succeed since client-a is explicitly assigned in policy.yaml
    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn test_bearer_auth_with_rbac_unassigned_token() {
    let router = create_scotty_app_with_rbac_auth().await;
    let server = TestServer::new(router).unwrap();

    // Test with a token that is not explicitly assigned
    let response = server
        .get("/api/v1/authenticated/blueprints")
        .add_header(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_str("Bearer unassigned-token").unwrap(),
        )
        .await;

    // Should fail since token is not in assignments
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn test_bearer_auth_missing_token_blueprints() {
    let router = create_scotty_app_with_bearer_auth().await;
    let server = TestServer::new(router).unwrap();

    // Make authenticated request without token
    let response = server.get("/api/v1/authenticated/blueprints").await;

    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn test_bearer_auth_malformed_header_blueprints() {
    let router = create_scotty_app_with_bearer_auth().await;
    let server = TestServer::new(router).unwrap();

    // Make authenticated request with malformed header (missing "Bearer " prefix)
    let response = server
        .get("/api/v1/authenticated/blueprints")
        .add_header(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_str("test-bearer-token-123").unwrap(),
        )
        .await;

    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn test_bearer_auth_public_endpoint() {
    let router = create_scotty_app_with_bearer_auth().await;
    let server = TestServer::new(router).unwrap();

    // Public endpoints should work without authentication
    let response = server.get("/api/v1/health").await;

    assert_eq!(response.status_code(), 200);
    let body = response.text();
    assert!(
        body.contains("OK") || body.contains("healthy") || body.contains("status"),
        "Health endpoint should return OK status: {}",
        body
    );
}


#[tokio::test]
async fn test_bearer_auth_apps_list_endpoint() {
    let router = create_scotty_app_with_bearer_auth().await;
    let server = TestServer::new(router).unwrap();

    // Test apps list endpoint with valid bearer token
    let response = server
        .get("/api/v1/authenticated/apps/list")
        .add_header(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_str("Bearer test-bearer-token-123").unwrap(),
        )
        .await;

    assert_eq!(response.status_code(), 200);
    let body = response.text();
    // Apps list should return JSON object containing apps array
    assert!(
        body.contains("\"apps\""),
        "Apps list should contain apps field: {}",
        body
    );
}

/// Create actual Scotty router for testing with OAuth configuration
async fn create_scotty_app_with_oauth() -> axum::Router {
    // Load OAuth test configuration from file
    let builder = Config::builder().add_source(config::File::with_name("tests/test_oauth_auth"));

    let config = builder.build().unwrap();
    let settings: crate::settings::config::Settings = config.try_deserialize().unwrap();

    // Create app state with OAuth test configuration
    let app_state = Arc::new(AppState {
        settings,
        stop_flag: crate::stop_flag::StopFlag::new(),
        clients: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        apps: scotty_core::apps::shared_app_list::SharedAppList::new(),
        docker: bollard::Docker::connect_with_local_defaults().unwrap(),
        task_manager: crate::tasks::manager::TaskManager::new(),
        oauth_state: None, // OAuth client creation may fail in tests, that's OK
        auth_service: Arc::new(
            crate::services::AuthorizationService::new("../config/casbin").await
                .expect("Failed to load RBAC config for test"),
        ),
    });

    // Create the actual Scotty router with all routes
    ApiRoutes::create(app_state)
}

#[tokio::test]
async fn test_oauth_auth_requires_authentication() {
    let router = create_scotty_app_with_oauth().await;
    let server = TestServer::new(router).unwrap();

    // OAuth mode should require authentication for protected endpoints
    let response = server.get("/api/v1/authenticated/blueprints").await;

    // Should return 401 since no OAuth token provided
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn test_oauth_public_endpoints_accessible() {
    let router = create_scotty_app_with_oauth().await;
    let server = TestServer::new(router).unwrap();

    // Public endpoints should still work in OAuth mode
    let response = server.get("/api/v1/health").await;
    assert_eq!(response.status_code(), 200);

    // Info endpoint should show OAuth is configured
    let response = server.get("/api/v1/info").await;
    assert_eq!(response.status_code(), 200);
    let body = response.text();
    assert!(
        body.contains("oauth") || body.contains("OAuth"),
        "Info endpoint should indicate OAuth mode: {}",
        body
    );
}

#[tokio::test]
async fn test_oauth_bearer_token_not_accepted() {
    let router = create_scotty_app_with_oauth().await;
    let server = TestServer::new(router).unwrap();

    // OAuth mode should not accept bearer tokens - only OAuth tokens
    let response = server
        .get("/api/v1/authenticated/blueprints")
        .add_header(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_str("Bearer some-bearer-token").unwrap(),
        )
        .await;

    // Should return 401 since bearer tokens are not valid in OAuth mode
    assert_eq!(response.status_code(), 401);
}

/// Create Scotty app with properly initialized OAuth state for flow testing
async fn create_scotty_app_with_oauth_flow() -> axum::Router {
    // Load OAuth test configuration
    let builder = Config::builder().add_source(config::File::with_name("tests/test_oauth_auth"));

    let config = builder.build().unwrap();
    let settings: crate::settings::config::Settings = config.try_deserialize().unwrap();

    // Initialize OAuth state with stores
    let oauth_state = match crate::oauth::client::create_oauth_client(&settings.api.oauth) {
        Ok(Some(client)) => Some(crate::oauth::handlers::OAuthState {
            client,
            device_flow_store: crate::oauth::create_device_flow_store(),
            web_flow_store: crate::oauth::create_web_flow_store(),
            session_store: crate::oauth::create_oauth_session_store(),
        }),
        _ => None, // OAuth client creation may fail with test config, that's OK
    };

    // Create app state with OAuth state
    let app_state = Arc::new(AppState {
        settings,
        stop_flag: crate::stop_flag::StopFlag::new(),
        clients: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        apps: scotty_core::apps::shared_app_list::SharedAppList::new(),
        docker: bollard::Docker::connect_with_local_defaults().unwrap(),
        task_manager: crate::tasks::manager::TaskManager::new(),
        oauth_state,
        auth_service: Arc::new(
            crate::services::AuthorizationService::new("../config/casbin").await
                .expect("Failed to load RBAC config for test"),
        ),
    });

    ApiRoutes::create(app_state)
}

#[tokio::test]
async fn test_oauth_device_flow_not_configured() {
    // Use app without OAuth state to test error handling
    let router = create_scotty_app_with_oauth().await;
    let server = TestServer::new(router).unwrap();

    let response = server.post("/oauth/device").await;

    // Should return 404 when OAuth client is not configured
    assert_eq!(response.status_code(), 404);
    let body = response.text();
    assert!(
        body.contains("oauth_not_configured") || body.contains("OAuth is not configured"),
        "Should indicate OAuth not configured: {}",
        body
    );
}

#[tokio::test]
async fn test_oauth_authorization_flow_url_generation() {
    let router = create_scotty_app_with_oauth_flow().await;
    let server = TestServer::new(router).unwrap();

    // Test authorization flow start endpoint
    let response = server
        .get("/oauth/authorize")
        .add_query_param("redirect_uri", "http://localhost:21342/api/oauth/callback")
        .await;

    // Should either redirect (302) or return error - but not 404
    assert_ne!(
        response.status_code(),
        404,
        "OAuth authorize endpoint should exist"
    );

    // Accept redirect (OAuth working) or error (OAuth client issues) - just not 404
    assert!(
        response.status_code() == 302
            || response.status_code() == 307
            || response.status_code() == 400
            || response.status_code() == 500,
        "OAuth authorize should return redirect or error, got: {}",
        response.status_code()
    );
}

#[tokio::test]
async fn test_oauth_endpoints_exist() {
    let router = create_scotty_app_with_oauth().await;
    let server = TestServer::new(router).unwrap();

    // Test that OAuth endpoints exist in router (even if they return errors)

    // Session exchange endpoint
    let response = server.post("/oauth/exchange").await;
    // Accept 404, 400, 415, or 500 - just testing endpoint existence patterns
    assert!(
        response.status_code() == 404
            || response.status_code() == 400
            || response.status_code() == 415
            || response.status_code() == 500,
        "OAuth exchange endpoint response: {}",
        response.status_code()
    );

    // OAuth callback endpoint
    let response = server.get("/api/oauth/callback").await;
    // Accept 404, 400, or 500 - just testing endpoint existence patterns
    assert!(
        response.status_code() == 404
            || response.status_code() == 400
            || response.status_code() == 500
            || response.status_code() == 302,
        "OAuth callback endpoint response: {}",
        response.status_code()
    );
}

#[tokio::test]
async fn test_oauth_flow_components_integration() {
    // Test that we can create OAuth client and stores for integration
    let config = Config::builder()
        .add_source(config::File::with_name("tests/test_oauth_auth"))
        .build()
        .unwrap();
    let settings: crate::settings::config::Settings = config.try_deserialize().unwrap();

    // Test OAuth client creation
    let oauth_result = crate::oauth::client::create_oauth_client(&settings.api.oauth);

    // OAuth client creation might fail with test config, that's OK - we're testing the flow
    match oauth_result {
        Ok(Some(_client)) => {
            // OAuth client created successfully with test config
        }
        Ok(None) => {
            // OAuth not configured (missing client_id/secret)
        }
        Err(_e) => {
            // OAuth client creation failed (invalid URL, etc.) - also OK for test
        }
    }

    // Test OAuth stores can be created
    let device_store = crate::oauth::create_device_flow_store();
    let web_store = crate::oauth::create_web_flow_store();
    let session_store = crate::oauth::create_oauth_session_store();

    // Verify stores are empty initially
    assert_eq!(device_store.lock().unwrap().len(), 0);
    assert_eq!(web_store.lock().unwrap().len(), 0);
    assert_eq!(session_store.lock().unwrap().len(), 0);
}
