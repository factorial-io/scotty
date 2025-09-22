#[cfg(test)]
mod tests {
    use super::super::login::{login_handler, validate_token_handler, FormData};
    use crate::app_state::AppState;
    use crate::services::AuthorizationService;
    use axum::{extract::State, response::IntoResponse, Json};
    use config::Config;
    use scotty_core::settings::api_server::AuthMode;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// Create a test AppState with mock settings for different auth modes
    async fn create_test_app_state(auth_mode: AuthMode) -> Arc<AppState> {
        // Use the test bearer auth config as base and override the auth mode
        let builder = Config::builder()
            .add_source(config::File::with_name("tests/test_bearer_auth"))
            .set_override(
                "api.auth_mode",
                match auth_mode {
                    AuthMode::Development => "dev",
                    AuthMode::OAuth => "oauth",
                    AuthMode::Bearer => "bearer",
                },
            )
            .unwrap();

        let config = builder.build().expect("Failed to build test config");
        let settings: crate::settings::config::Settings = config
            .try_deserialize()
            .expect("Failed to deserialize settings");

        // Create authorization service
        let auth_service = Arc::new(
            AuthorizationService::new("../config/casbin")
                .await
                .expect("Failed to create auth service"),
        );

        let docker = bollard::Docker::connect_with_local_defaults().unwrap();
        Arc::new(AppState {
            settings,
            stop_flag: crate::stop_flag::StopFlag::new(),
            clients: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            apps: scotty_core::apps::shared_app_list::SharedAppList::new(),
            docker: docker.clone(),
            task_manager: crate::tasks::manager::TaskManager::new(),
            oauth_state: None,
            auth_service,
            logs_service: crate::docker::services::logs::LogStreamingService::new(docker),
        })
    }

    #[tokio::test]
    async fn test_login_bearer_mode_with_valid_token() {
        let app_state = create_test_app_state(AuthMode::Bearer).await;

        // Test with admin token that has RBAC assignments (from test config)
        let form_data = FormData {
            password: "test-bearer-token-123".to_string(), // admin token from test config
        };

        let response = login_handler(State(app_state), Json(form_data)).await;
        let body = response.into_response().into_body();

        // Convert response to JSON for assertions
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(json["status"], "success");
        assert_eq!(json["auth_mode"], "bearer");
        assert_eq!(json["token"], "test-bearer-token-123");
    }

    #[tokio::test]
    async fn test_login_bearer_mode_with_invalid_token() {
        let app_state = create_test_app_state(AuthMode::Bearer).await;

        // Test with completely invalid token
        let form_data = FormData {
            password: "completely-invalid-token".to_string(),
        };

        let response = login_handler(State(app_state), Json(form_data)).await;
        let body = response.into_response().into_body();

        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(json["status"], "error");
        assert_eq!(json["auth_mode"], "bearer");
        assert_eq!(json["message"], "Invalid token");
    }

    #[tokio::test]
    async fn test_login_bearer_mode_token_without_rbac() {
        let app_state = create_test_app_state(AuthMode::Bearer).await;

        // Test with no-rbac token that has no RBAC assignments
        let form_data = FormData {
            password: "token-without-rbac-assignments".to_string(), // no-rbac token from test config
        };

        let response = login_handler(State(app_state), Json(form_data)).await;
        let body = response.into_response().into_body();

        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        // Should fail because no RBAC assignments
        assert_eq!(json["status"], "error");
        assert_eq!(json["message"], "Invalid token");
    }

    #[tokio::test]
    async fn test_login_dev_mode() {
        let app_state = create_test_app_state(AuthMode::Development).await;

        // In dev mode, any password should work
        let form_data = FormData {
            password: "anything".to_string(),
        };

        let response = login_handler(State(app_state), Json(form_data)).await;
        let body = response.into_response().into_body();

        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(json["status"], "success");
        assert_eq!(json["auth_mode"], "dev");
        assert!(json["message"]
            .as_str()
            .unwrap()
            .contains("Development mode"));
    }

    #[tokio::test]
    async fn test_login_oauth_mode() {
        let app_state = create_test_app_state(AuthMode::OAuth).await;

        // OAuth mode should return redirect
        let form_data = FormData {
            password: "".to_string(),
        };

        let response = login_handler(State(app_state), Json(form_data)).await;
        let body = response.into_response().into_body();

        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(json["status"], "redirect");
        assert_eq!(json["auth_mode"], "oauth");
        assert_eq!(json["redirect_url"], "/oauth/authorize");
    }

    #[tokio::test]
    async fn test_validate_token_handler() {
        // validate_token_handler just returns success if the middleware lets it through
        let response = validate_token_handler().await;
        let body = response.into_response().into_body();

        let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(json["status"], "success");
    }
}
