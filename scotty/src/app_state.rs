use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
};

use bollard::Docker;
use scotty_core::apps::shared_app_list::SharedAppList;
use scotty_core::settings::api_server::AuthMode;
use scotty_core::settings::docker::DockerConnectOptions;
use secrecy::ExposeSecret;
use tokio::sync::{broadcast, Mutex};
use tracing::{info, warn};

use crate::api::basic_auth::CurrentUser;
use crate::api::websocket::WebSocketMessenger;
use crate::docker::services::logs::LogStreamingService;
use crate::docker::services::shell::ShellService;
use crate::oauth::handlers::OAuthState;
use crate::oauth::{self, create_oauth_session_store, create_web_flow_store};
use crate::services::{authorization::fallback::FallbackService, AuthorizationService};
use crate::settings::config::Settings;
use crate::stop_flag;
use crate::tasks::manager;
use crate::tasks::output_streaming::TaskOutputStreamingService;

#[derive(Debug, Clone)]
pub struct WebSocketClient {
    pub sender: broadcast::Sender<axum::extract::ws::Message>,
    pub user: Option<CurrentUser>,
}

impl WebSocketClient {
    pub fn new(sender: broadcast::Sender<axum::extract::ws::Message>) -> Self {
        Self { sender, user: None }
    }

    pub fn authenticate(&mut self, user: CurrentUser) {
        self.user = Some(user);
    }

    #[allow(dead_code)]
    pub fn is_authenticated(&self) -> bool {
        self.user.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub settings: Settings,
    pub stop_flag: stop_flag::StopFlag,
    pub apps: SharedAppList,
    pub docker: Docker,
    pub task_manager: manager::TaskManager,
    pub oauth_state: Option<OAuthState>,
    pub auth_service: Arc<AuthorizationService>,
    pub logs_service: LogStreamingService,
    pub shell_service: ShellService,
    pub task_output_service: TaskOutputStreamingService,
    pub messenger: WebSocketMessenger,
    /// Unix timestamp (seconds) of the next scheduled app-state sweep, `0` when no
    /// sweep has run yet. Server-global on purpose: a single scheduler job inspects
    /// every app, so there is no per-app next-check time. Written by the sweep, read
    /// via [`AppState::next_app_check_at`].
    pub next_app_check: Arc<AtomicI64>,
    /// The OAuth server's database (`api.database_path`), open in oauth mode.
    pub oauth_db: Option<sqlx::SqlitePool>,
    /// The grantry session server (D2), built over `oauth_db` in oauth mode.
    pub session_server: Option<oauth::server::SessionServer>,
    /// The grantry mcp server (D2), over the same database and secret.
    pub mcp_server: Option<oauth::server::McpServer>,
    /// Apps with a create in flight.
    pub create_locks: CreateLocks,
}

/// Apps (by directory name) with a create in flight.
///
/// Create is create-or-update: it reads the existing app from disk, checks
/// rights against it and keeps its generated secrets. Two creates of the
/// same app running at once would both decide on the same old state and
/// each generate their own secrets, the last one winning.
#[derive(Debug, Clone, Default)]
pub struct CreateLocks(Arc<std::sync::Mutex<std::collections::HashSet<String>>>);

impl CreateLocks {
    /// Lock `app_dir`, or `None` if a create of it is already running.
    /// The lock is released when the returned guard is dropped.
    pub fn try_lock(&self, app_dir: &str) -> Option<CreateLock> {
        let mut locked = self.0.lock().unwrap_or_else(|e| e.into_inner());
        locked.insert(app_dir.to_string()).then(|| CreateLock {
            locks: self.clone(),
            app_dir: app_dir.to_string(),
        })
    }
}

/// Holds the create lock of one app; see [`CreateLocks`].
#[derive(Debug)]
pub struct CreateLock {
    locks: CreateLocks,
    app_dir: String,
}

impl Drop for CreateLock {
    fn drop(&mut self) {
        let mut locked = self.locks.0.lock().unwrap_or_else(|e| e.into_inner());
        locked.remove(&self.app_dir);
    }
}

pub type SharedAppState = Arc<AppState>;

impl AppState {
    /// Records when the next scheduled app-state sweep is due.
    pub fn set_next_app_check(&self, at: chrono::DateTime<chrono::Local>) {
        self.next_app_check.store(at.timestamp(), Ordering::Relaxed);
    }

    /// The next scheduled app-state sweep, or `None` if no sweep has run yet.
    pub fn next_app_check_at(&self) -> Option<chrono::DateTime<chrono::Local>> {
        match self.next_app_check.load(Ordering::Relaxed) {
            0 => None,
            timestamp => chrono::DateTime::from_timestamp(timestamp, 0)
                .map(|dt| dt.with_timezone(&chrono::Local)),
        }
    }

    pub async fn new() -> anyhow::Result<SharedAppState> {
        let settings = Settings::new()?;

        let stop_flag = stop_flag::StopFlag::new();
        stop_flag::register_signal_handler(&stop_flag);

        let docker = match &settings.docker.connection {
            DockerConnectOptions::Local => Docker::connect_with_local_defaults()?,
            DockerConnectOptions::Socket => Docker::connect_with_socket_defaults()?,
            DockerConnectOptions::Http => Docker::connect_with_http_defaults()?,
        };

        // Initialize shared log streaming service
        let logs_service = LogStreamingService::new(docker.clone());

        // Initialize shared shell service
        let shell_service = ShellService::new(docker.clone(), settings.shell.clone());

        // Initialize OAuth if configured
        let oauth_state = match oauth::client::create_oauth_client(&settings.api.oauth) {
            Ok(Some(client)) => {
                tracing::info!("OAuth client initialized");
                Some(OAuthState {
                    client,
                    web_flow_store: create_web_flow_store(),
                    session_store: create_oauth_session_store(),
                })
            }
            Ok(None) => {
                tracing::info!("OAuth not configured");
                None
            }
            Err(e) => {
                tracing::error!("Failed to create OAuth client: {}", e);
                None
            }
        };

        let oauth_db = match settings.api.auth_mode {
            AuthMode::OAuth => Some(oauth::store::open(&settings.api.database_path).await?),
            _ => None,
        };

        // Initialize authorization service (always available with fallback)
        let auth_service = Arc::new(match AuthorizationService::new("config/casbin").await {
            Ok(service) => {
                info!("Authorization service loaded successfully from config");
                service
            }
            Err(e) => {
                warn!(
                    "Failed to load authorization config from 'config/casbin': {}. Falling back to default configuration with view-only permissions.",
                    e
                );
                FallbackService::create_fallback_service(settings.api.access_token.clone()).await
            }
        });

        let (session_server, mcp_server) = match (&oauth_db, &settings.api.jwt_secret) {
            (Some(pool), Some(secret)) => (
                Some(oauth::server::session_server(
                    pool.clone(),
                    secret.expose_secret().as_bytes(),
                    auth_service.clone(),
                )),
                Some(oauth::server::mcp_server(
                    pool.clone(),
                    secret.expose_secret().as_bytes(),
                    auth_service.clone(),
                    &settings.api.public_base_url(),
                )),
            ),
            _ => (None, None),
        };

        // Create WebSocket clients and messenger
        let clients = Arc::new(Mutex::new(HashMap::new()));
        let messenger = WebSocketMessenger::new(clients.clone());

        let state = Arc::new(AppState {
            settings,
            stop_flag: stop_flag.clone(),
            apps: SharedAppList::new(),
            docker,
            task_manager: manager::TaskManager::new(messenger.clone(), auth_service.clone()),
            oauth_state,
            auth_service,
            logs_service,
            shell_service,
            task_output_service: TaskOutputStreamingService::new(),
            messenger,
            next_app_check: Arc::new(AtomicI64::new(0)),
            oauth_db,
            session_server,
            mcp_server,
            create_locks: CreateLocks::default(),
        });

        Ok(state)
    }

    pub async fn new_for_config_only() -> anyhow::Result<SharedAppState> {
        let settings = Settings::new()?;
        let docker = Docker::connect_with_local_defaults()?;

        let clients = Arc::new(Mutex::new(HashMap::new()));
        let messenger = WebSocketMessenger::new(clients.clone());
        let auth_service = Arc::new(AuthorizationService::create_fallback_service(None).await);

        Ok(Arc::new(AppState {
            settings: settings.clone(),
            stop_flag: stop_flag::StopFlag::new(),
            apps: SharedAppList::new(),
            docker: docker.clone(),
            task_manager: manager::TaskManager::new(messenger.clone(), auth_service.clone()),
            oauth_state: None,
            auth_service,
            logs_service: LogStreamingService::new(docker.clone()),
            shell_service: ShellService::new(docker, settings.shell.clone()),
            task_output_service: TaskOutputStreamingService::new(),
            messenger,
            next_app_check: Arc::new(AtomicI64::new(0)),
            oauth_db: None,
            create_locks: CreateLocks::default(),
            session_server: None,
            mcp_server: None,
        }))
    }
}

#[cfg(test)]
mod create_lock_tests {
    use super::CreateLocks;

    #[test]
    fn one_create_per_app_at_a_time() {
        let locks = CreateLocks::default();
        let first = locks.try_lock("demo").expect("free");
        assert!(locks.try_lock("demo").is_none(), "second create must wait");
        assert!(
            locks.try_lock("other").is_some(),
            "other apps are independent"
        );
        drop(first);
        assert!(locks.try_lock("demo").is_some(), "released on drop");
    }
}
