use scottyctl::commands::auth::auth_status;
use scottyctl::context::{AppContext, ServerSettings};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test that auth:status returns error when OAuth token is invalid (401 response)
#[tokio::test]
async fn test_auth_status_with_invalid_oauth_token_returns_error() {
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

    // Create app context with mock server URL and a fake OAuth token
    let server_settings = ServerSettings {
        server: mock_server.uri(),
        access_token: Some("invalid_oauth_token".to_string()),
    };
    
    let app_context = AppContext::new(server_settings);

    // Call auth_status - should return Err because token is invalid
    let result = auth_status(&app_context).await;

    // Verify it returns an error
    assert!(
        result.is_err(),
        "auth_status should return Err when token is invalid (401)"
    );
    
    // Verify error message suggests re-authentication
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("invalid") || error_msg.contains("expired"),
        "Error message should mention token is invalid or expired, got: {}",
        error_msg
    );
}

/// Test that auth:status succeeds when token is valid
#[tokio::test]
async fn test_auth_status_with_valid_token_succeeds() {
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
