//! The authorization middleware must refuse with a JSON body that explains
//! why, so clients can surface the reason instead of failing silently.

use axum_test::TestServer;
use scotty::api::router::ApiRoutes;
use scotty::api::test_utils::create_test_app_state_with_config;

#[tokio::test]
async fn denied_action_returns_403_with_json_message() {
    let app_state = create_test_app_state_with_config("tests/test_bearer_auth", None).await;
    let server = TestServer::new(ApiRoutes::create(app_state));

    // `client-a` is a developer in scope client-a only; the app is unknown, so
    // it has no scope the user can manage.
    let response = server
        .get("/api/v1/authenticated/apps/run/some-other-app")
        .add_header(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_str("Bearer client-a-secure-token-456").unwrap(),
        )
        .await;

    assert_eq!(response.status_code(), 403);
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"], true);
    let message = body["message"].as_str().unwrap();
    assert!(
        message.contains("lacks manage permission"),
        "unexpected message: {message}"
    );
}
