//! Shared test utilities for API tests
//!
//! This module provides common helper functions used across multiple test files
//! to reduce code duplication.

use crate::api::websocket::WebSocketMessenger;
use crate::app_state::AppState;
use config::Config;
use std::sync::Arc;

/// Create a Docker client for testing
///
/// Tries local socket first, falls back to HTTP connection for environments
/// without a Docker socket (e.g. CI).
#[allow(dead_code)]
pub fn create_test_docker_client() -> bollard::Docker {
    bollard::Docker::connect_with_local_defaults()
        .or_else(|_| bollard::Docker::connect_with_http_defaults())
        .unwrap()
}

/// Initialise the global metrics recorder once per test process.
///
/// Production does this in `init_telemetry_and_tracing`. Integration tests
/// link the crate without `cfg(test)`, so `metrics()` has no no-op fallback
/// there and panics on the first task or log stream unless this ran. Unit
/// tests keep the no-op fallback and skip this.
fn init_test_metrics() {
    #[cfg(all(any(feature = "telemetry-grpc", feature = "telemetry-http"), not(test)))]
    {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            // Warn like production does; panicking here would poison the Once
            // and hide the cause behind "previously poisoned" in later tests.
            if let Err(e) = crate::metrics::init_metrics() {
                tracing::warn!("Failed to initialize metrics: {e}");
            }
        });
    }
}

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
    // Use the tracked fixture rather than the runtime config/casbin (which is
    // gitignored and seeded at runtime), so tests have deterministic
    // assignments and never race on seeding.
    create_test_app_state_with_policy_dir(settings, oauth_state, "tests/fixtures/casbin").await
}

/// Create test AppState with provided settings and a Casbin policy directory
///
/// The directory must contain `model.conf` and `policy.yaml`. It is copied
/// into a fresh temp dir first, so calls that change the policy (for example
/// `set_app_scopes`) write to the copy and never to the given directory, which
/// may be a tracked fixture.
#[allow(dead_code)]
pub async fn create_test_app_state_with_policy_dir(
    settings: crate::settings::config::Settings,
    oauth_state: Option<crate::oauth::handlers::OAuthState>,
    policy_dir: &str,
) -> Arc<AppState> {
    init_test_metrics();
    let docker = create_test_docker_client();
    // ponytail: the copy is never deleted (keep()), like the oauth SQLite dir
    // below; hand the TempDir back to callers if $TMPDIR growth matters.
    let policy_copy = tempfile::tempdir().unwrap().keep();
    for entry in std::fs::read_dir(policy_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            std::fs::copy(entry.path(), policy_copy.join(entry.file_name())).unwrap();
        }
    }
    let auth_service = Arc::new(
        crate::services::AuthorizationService::new(policy_copy.to_str().unwrap())
            .await
            .expect("Failed to load RBAC config for test"),
    );

    // In oauth mode, a session server over a fresh SQLite file.
    let (oauth_db, session_server, mcp_server) =
        match (&settings.api.auth_mode, &settings.api.jwt_secret) {
            (scotty_core::settings::api_server::AuthMode::OAuth, Some(secret)) => {
                use secrecy::ExposeSecret;
                // ponytail: the temp dir is never deleted (keep()).
                let dir = tempfile::tempdir().unwrap().keep();
                let pool = crate::oauth::store::open(dir.join("scotty.sqlite"))
                    .await
                    .unwrap();
                let server = crate::oauth::server::session_server(
                    pool.clone(),
                    secret.expose_secret().as_bytes(),
                    auth_service.clone(),
                );
                let mcp = crate::oauth::server::mcp_server(
                    pool.clone(),
                    secret.expose_secret().as_bytes(),
                    auth_service.clone(),
                    &settings.api.public_base_url(),
                );
                (Some(pool), Some(server), Some(mcp))
            }
            _ => (None, None, None),
        };

    Arc::new(AppState {
        stop_flag: crate::stop_flag::StopFlag::new(),
        messenger: create_test_websocket_messenger(),
        apps: scotty_core::apps::shared_app_list::SharedAppList::new(),
        docker: docker.clone(),
        task_manager: crate::tasks::manager::TaskManager::new(
            create_test_websocket_messenger(),
            auth_service.clone(),
        ),
        oauth_state,
        auth_service,
        logs_service: crate::docker::services::logs::LogStreamingService::new(docker.clone()),
        shell_service: crate::docker::services::shell::ShellService::new(
            docker,
            settings.shell.clone(),
        ),
        task_output_service: crate::tasks::output_streaming::TaskOutputStreamingService::new(),
        settings,
        next_app_check: Arc::new(std::sync::atomic::AtomicI64::new(0)),
        create_locks: Default::default(),
        oauth_db,
        session_server,
        mcp_server,
    })
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn policy_changes_do_not_touch_the_tracked_fixture() {
        let fixture = "tests/fixtures/casbin/policy.yaml";
        let before = std::fs::read(fixture).unwrap();
        let state = super::create_test_app_state_with_config("tests/test_bearer_auth", None).await;
        state
            .auth_service
            .set_app_scopes("issue-944-app", vec!["client-a".to_string()])
            .await
            .unwrap();
        assert_eq!(std::fs::read(fixture).unwrap(), before);
    }
}
