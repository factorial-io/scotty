use scottyctl::commands::auth::auth_status;
use scottyctl::context::{AppContext, ServerSettings};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test that auth:status returns error when Bearer token is invalid (401 response)
#[tokio::test]
async fn test_auth_status_with_invalid_bearer_token_unauthorized() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Mock the /api/v1/authenticated/scopes/list endpoint to return 401
    Mock::given(method("GET"))
        .and(path("/api/v1/authenticated/scopes/list"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": "Unauthorized"
        })))
        .mount(&mock_server)
        .await;

    // Create app context with mock server URL and an invalid Bearer token
    let server_settings = ServerSettings {
        server: mock_server.uri(),
        access_token: Some("invalid_bearer_token".to_string()),
    };

    let app_context = AppContext::new(server_settings);

    // Call auth_status - should return Err because token is invalid
    let result = auth_status(&app_context).await;

    // Verify it returns an error
    assert!(
        result.is_err(),
        "auth_status should return Err when token is invalid (401)"
    );

    // Verify error message mentions Bearer token and SCOTTY_ACCESS_TOKEN
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Bearer token") && error_msg.contains("SCOTTY_ACCESS_TOKEN"),
        "Error message should mention Bearer token and SCOTTY_ACCESS_TOKEN for Bearer auth, got: {}",
        error_msg
    );
}

/// Test that auth:status succeeds when token is valid
#[tokio::test]
async fn test_auth_status_with_valid_bearer_token_succeeds() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Mock the /api/v1/authenticated/scopes/list endpoint to return success
    Mock::given(method("GET"))
        .and(path("/api/v1/authenticated/scopes/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "scopes": [
                {
                    "name": "default",
                    "description": "Default scope",
                    "permissions": ["view"]
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    // Create app context with mock server URL and a valid token
    let server_settings = ServerSettings {
        server: mock_server.uri(),
        access_token: Some("valid_token".to_string()),
    };

    let app_context = AppContext::new(server_settings);

    // Call auth_status - should succeed
    let result = auth_status(&app_context).await;

    // Verify it returns Ok
    assert!(
        result.is_ok(),
        "auth_status should return Ok when token is valid"
    );
}

/// Test that auth:status returns error when Bearer token is invalid (403 Forbidden)
#[tokio::test]
async fn test_auth_status_with_invalid_bearer_token_forbidden() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Mock the /api/v1/authenticated/scopes/list endpoint to return 403
    Mock::given(method("GET"))
        .and(path("/api/v1/authenticated/scopes/list"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "error": "Forbidden"
        })))
        .mount(&mock_server)
        .await;

    // Create app context with mock server URL and an invalid Bearer token
    let server_settings = ServerSettings {
        server: mock_server.uri(),
        access_token: Some("invalid_bearer_token".to_string()),
    };

    let app_context = AppContext::new(server_settings);

    // Call auth_status - should return Err because token is invalid
    let result = auth_status(&app_context).await;

    // Verify it returns an error
    assert!(
        result.is_err(),
        "auth_status should return Err when token is invalid (403)"
    );

    // Verify error message mentions Bearer token and SCOTTY_ACCESS_TOKEN
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Bearer token") && error_msg.contains("SCOTTY_ACCESS_TOKEN"),
        "Error message should mention Bearer token and SCOTTY_ACCESS_TOKEN, got: {}",
        error_msg
    );
}

/// Test that auth:status succeeds with no authentication (exit code 0)
#[tokio::test]
async fn test_auth_status_with_no_auth_succeeds() {
    // Start mock server (won't be called since no auth)
    let mock_server = MockServer::start().await;

    // Create app context with NO access token
    let server_settings = ServerSettings {
        server: mock_server.uri(),
        access_token: None,
    };

    let app_context = AppContext::new(server_settings);

    // Call auth_status - should succeed (but show "not authenticated")
    let result = auth_status(&app_context).await;

    // Verify it returns Ok (exit code 0)
    assert!(
        result.is_ok(),
        "auth_status should return Ok (exit 0) when not authenticated"
    );
}
