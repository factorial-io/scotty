//! The create endpoint's 413 must name the actual body size and the setting
//! that caps it — axum's stock rejection only says "length limit exceeded".

use axum_test::TestServer;
use scotty::api::router::ApiRoutes;
use scotty::api::test_utils::create_test_app_state_with_config;

/// Config identical to the bearer-auth one except `create_app_max_size: 1K`.
async fn make_router() -> axum::Router {
    let app_state = create_test_app_state_with_config("tests/test_create_body_limit", None).await;
    ApiRoutes::create(app_state)
}

#[tokio::test]
async fn oversized_create_payload_reports_size_and_setting() {
    let server = TestServer::new(make_router().await);

    let response = server
        .post("/api/v1/authenticated/apps/create")
        .authorization_bearer("test-bearer-token-123")
        .json(&serde_json::json!({
            "app_name": "too-big",
            "settings": {},
            "files": { "files": [
                { "name": "compose.yml", "content": "a".repeat(4096), "compressed": false }
            ]},
        }))
        .await;

    assert_eq!(response.status_code(), 413);
    let body = response.text();
    assert!(body.contains("create_app_max_size"), "{body}");
    assert!(body.contains("1.00 KB"), "{body}");
}
