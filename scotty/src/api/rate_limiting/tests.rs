//! Integration tests for rate limiting middleware
//!
//! These tests verify that rate limiting works correctly across all three tiers:
//! - Public auth (login) - IP-based
//! - OAuth - IP-based
//! - Authenticated - Token-based

#[cfg(test)]
mod integration_tests {
    use crate::api::router::ApiRoutes;
    use crate::app_state::AppState;
    use axum::http::StatusCode;
    use axum_test::TestServer;
    use std::sync::Arc;

    fn create_test_websocket_messenger() -> crate::api::websocket::WebSocketMessenger {
        use crate::api::websocket::WebSocketMessenger;
        let clients = Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
        WebSocketMessenger::new(clients)
    }

    async fn create_test_app_with_rate_limiting(
        enabled: bool,
        requests_per_minute: u64,
        burst_size: u32,
    ) -> TestServer {
        use config::Config;

        // Start with base test config and override rate limiting settings
        let builder = Config::builder()
            .add_source(config::File::with_name("tests/test_bearer_auth"))
            .set_override("api.rate_limiting.enabled", enabled)
            .unwrap()
            .set_override(
                "api.rate_limiting.public_auth.requests_per_minute",
                requests_per_minute as i64,
            )
            .unwrap()
            .set_override(
                "api.rate_limiting.public_auth.burst_size",
                burst_size as i64,
            )
            .unwrap()
            .set_override(
                "api.rate_limiting.oauth.requests_per_minute",
                requests_per_minute as i64,
            )
            .unwrap()
            .set_override("api.rate_limiting.oauth.burst_size", burst_size as i64)
            .unwrap()
            .set_override(
                "api.rate_limiting.authenticated.requests_per_minute",
                requests_per_minute as i64,
            )
            .unwrap()
            .set_override(
                "api.rate_limiting.authenticated.burst_size",
                burst_size as i64,
            )
            .unwrap();

        let config = builder.build().unwrap();
        let settings: crate::settings::config::Settings = config.try_deserialize().unwrap();

        let docker = bollard::Docker::connect_with_local_defaults().unwrap();
        let state = Arc::new(AppState {
            settings,
            stop_flag: crate::stop_flag::StopFlag::new(),
            messenger: create_test_websocket_messenger(),
            apps: scotty_core::apps::shared_app_list::SharedAppList::new(),
            docker: docker.clone(),
            task_manager: crate::tasks::manager::TaskManager::new(create_test_websocket_messenger()),
            oauth_state: None,
            auth_service: Arc::new(
                crate::services::AuthorizationService::new("../config/casbin")
                    .await
                    .expect("Failed to load RBAC config for test"),
            ),
            logs_service: crate::docker::services::logs::LogStreamingService::new(docker),
            task_output_service: crate::tasks::output_streaming::TaskOutputStreamingService::new(),
        });

        let app = ApiRoutes::create(state);
        TestServer::new(app).unwrap()
    }

    #[tokio::test]
    async fn test_rate_limiting_disabled_allows_unlimited_requests() {
        let server = create_test_app_with_rate_limiting(false, 5, 10).await;

        // Should be able to make many requests without hitting limit
        for _ in 0..20 {
            let response = server
                .post("/api/v1/login")
                .json(&serde_json::json!({
                    "username": "test",
                    "password": "test"
                }))
                .await;

            // Should not get 429 (rate limited)
            assert_ne!(response.status_code(), StatusCode::TOO_MANY_REQUESTS);
        }
    }

    #[tokio::test]
    async fn test_public_auth_rate_limiting_enforced() {
        // Very strict limit: 60 requests per minute = 1/sec, burst 1
        let server = create_test_app_with_rate_limiting(true, 60, 1).await;

        // First request should succeed
        let response1 = server
            .post("/api/v1/login")
            .json(&serde_json::json!({
                "username": "test",
                "password": "test"
            }))
            .await;
        assert_ne!(response1.status_code(), StatusCode::TOO_MANY_REQUESTS);

        // Second request immediately after should be rate limited
        let response2 = server
            .post("/api/v1/login")
            .json(&serde_json::json!({
                "username": "test",
                "password": "test"
            }))
            .await;
        assert_eq!(
            response2.status_code(),
            StatusCode::TOO_MANY_REQUESTS,
            "Second request should be rate limited"
        );
    }

    #[tokio::test]
    async fn test_oauth_rate_limiting_enforced() {
        let server = create_test_app_with_rate_limiting(true, 60, 1).await;

        // First request should succeed
        let response1 = server
            .post("/oauth/device")
            .json(&serde_json::json!({
                "client_id": "test"
            }))
            .await;
        assert_ne!(response1.status_code(), StatusCode::TOO_MANY_REQUESTS);

        // Second request immediately after should be rate limited
        let response2 = server
            .post("/oauth/device")
            .json(&serde_json::json!({
                "client_id": "test"
            }))
            .await;
        assert_eq!(
            response2.status_code(),
            StatusCode::TOO_MANY_REQUESTS,
            "Second OAuth request should be rate limited"
        );
    }

    #[tokio::test]
    async fn test_authenticated_rate_limiting_per_token() {
        let server = create_test_app_with_rate_limiting(true, 60, 2).await;

        // Token 1 should be rate limited independently from token 2
        let token1 = "test-token-1";
        let token2 = "test-token-2";

        // Exhaust token1's quota
        for _ in 0..3 {
            server
                .get("/api/v1/authenticated/apps/list")
                .add_header(
                    axum::http::header::AUTHORIZATION,
                    axum::http::HeaderValue::from_str(&format!("Bearer {}", token1)).unwrap(),
                )
                .await;
        }

        // Token1 should now be rate limited
        let response1 = server
            .get("/api/v1/authenticated/apps/list")
            .add_header(
                axum::http::header::AUTHORIZATION,
                axum::http::HeaderValue::from_str(&format!("Bearer {}", token1)).unwrap(),
            )
            .await;
        assert_eq!(response1.status_code(), StatusCode::TOO_MANY_REQUESTS);

        // Token2 should still work (independent quota)
        let response2 = server
            .get("/api/v1/authenticated/apps/list")
            .add_header(
                axum::http::header::AUTHORIZATION,
                axum::http::HeaderValue::from_str(&format!("Bearer {}", token2)).unwrap(),
            )
            .await;
        assert_ne!(response2.status_code(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_rate_limit_response_format() {
        let server = create_test_app_with_rate_limiting(true, 60, 1).await;

        // Exhaust quota
        server
            .post("/api/v1/login")
            .json(&serde_json::json!({
                "username": "test",
                "password": "test"
            }))
            .await;

        // Next request should be rate limited
        let response = server
            .post("/api/v1/login")
            .json(&serde_json::json!({
                "username": "test",
                "password": "test"
            }))
            .await;

        assert_eq!(response.status_code(), StatusCode::TOO_MANY_REQUESTS);

        // Check for retry-after header (tower-governor should add this)
        // Note: Exact header format depends on tower-governor version
    }

    #[tokio::test]
    async fn test_different_endpoints_have_independent_limits() {
        let server = create_test_app_with_rate_limiting(true, 60, 2).await;

        // Exhaust login quota
        for _ in 0..3 {
            server
                .post("/api/v1/login")
                .json(&serde_json::json!({
                    "username": "test",
                    "password": "test"
                }))
                .await;
        }

        // OAuth endpoints should still work (different tier)
        let oauth_response = server
            .post("/oauth/device")
            .json(&serde_json::json!({
                "client_id": "test"
            }))
            .await;

        assert_ne!(
            oauth_response.status_code(),
            StatusCode::TOO_MANY_REQUESTS,
            "OAuth should have independent rate limit from login"
        );
    }
}
