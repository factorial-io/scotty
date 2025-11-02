//! Shared test utilities for API tests
//!
//! This module provides common helper functions used across multiple test files
//! to reduce code duplication.

use crate::api::websocket::WebSocketMessenger;
use crate::app_state::AppState;
use config::Config;
use std::sync::Arc;

/// Create test WebSocket messenger
///
/// Creates a WebSocketMessenger with an empty client map for use in tests.
#[allow(dead_code)]
pub fn create_test_websocket_messenger() -> WebSocketMessenger {
    let clients = Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
    WebSocketMessenger::new(clients)
}

/// Create test AppState from config file
///
/// # Arguments
/// * `config_file` - Path to config file (relative to project root)
/// * `oauth_state` - Optional OAuth state for testing OAuth flows
///
/// # Returns
/// Arc-wrapped AppState configured for testing
#[allow(dead_code)]
pub async fn create_test_app_state_with_config(
    config_file: &str,
    oauth_state: Option<crate::oauth::handlers::OAuthState>,
) -> Arc<AppState> {
    // Load test configuration from file
    let builder = Config::builder().add_source(config::File::with_name(config_file));
    let config = builder.build().unwrap();
    let settings: crate::settings::config::Settings = config.try_deserialize().unwrap();

    create_test_app_state_with_settings(settings, oauth_state).await
}

/// Create test AppState with provided settings
///
/// # Arguments
/// * `settings` - Settings to use for the AppState
/// * `oauth_state` - Optional OAuth state for testing OAuth flows
///
/// # Returns
/// Arc-wrapped AppState configured for testing
#[allow(dead_code)]
pub async fn create_test_app_state_with_settings(
    settings: crate::settings::config::Settings,
    oauth_state: Option<crate::oauth::handlers::OAuthState>,
) -> Arc<AppState> {
    let docker = bollard::Docker::connect_with_local_defaults().unwrap();

    Arc::new(AppState {
        stop_flag: crate::stop_flag::StopFlag::new(),
        messenger: create_test_websocket_messenger(),
        apps: scotty_core::apps::shared_app_list::SharedAppList::new(),
        docker: docker.clone(),
        task_manager: crate::tasks::manager::TaskManager::new(create_test_websocket_messenger()),
        oauth_state,
        auth_service: Arc::new(
            crate::services::AuthorizationService::new("../config/casbin")
                .await
                .expect("Failed to load RBAC config for test"),
        ),
        logs_service: crate::docker::services::logs::LogStreamingService::new(docker.clone()),
        shell_service: crate::docker::services::shell::ShellService::new(
            docker,
            settings.shell.clone(),
        ),
        task_output_service: crate::tasks::output_streaming::TaskOutputStreamingService::new(),
        settings,
    })
}
