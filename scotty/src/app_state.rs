use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
};

use anyhow::Context;
use bollard::Docker;
use scotty_core::apps::app_data::AppOperation;
use scotty_core::apps::shared_app_list::SharedAppList;
use scotty_core::settings::api_server::AuthMode;
use scotty_core::settings::docker::DockerConnectOptions;
use scotty_core::utils::slugify::slugify;
use scotty_core::websocket::message::WebSocketMessage;
use secrecy::ExposeSecret;
use tokio::sync::{broadcast, Mutex};
use tracing::info;
use uuid::Uuid;

use crate::api::basic_auth::CurrentUser;
use crate::api::error::AppError;
use crate::api::websocket::WebSocketMessenger;
use crate::docker::services::logs::LogStreamingService;
use crate::docker::services::shell::ShellService;
use crate::oauth::handlers::OAuthState;
use crate::oauth::{self, create_oauth_session_store, create_web_flow_store};
use crate::services::authorization::{decision::Target, Permission};
use crate::services::AuthorizationService;
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
    /// The operation running on each app.
    pub app_locks: AppLocks,
}

/// At most one operation that changes an app at a time, keyed by the app's
/// directory name.
///
/// In memory only: a restart ends every operation, so a persisted lock could
/// only go stale.
#[derive(Debug, Clone, Default)]
pub struct AppLocks(Arc<std::sync::Mutex<HashMap<String, AppOperation>>>);

impl AppLocks {
    fn entries(&self) -> std::sync::MutexGuard<'_, HashMap<String, AppOperation>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Lock `app` for `operation`, or return the operation holding it. The
    /// lock is released when the returned guard is dropped.
    pub fn try_lock(&self, app: &str, operation: &str) -> Result<AppLock, AppOperation> {
        let key = slugify(app);
        let mut entries = self.entries();
        if let Some(holder) = entries.get(&key) {
            return Err(holder.clone());
        }
        entries.insert(
            key.clone(),
            AppOperation {
                operation: operation.to_string(),
                task_id: None,
                started_at: jiff::Timestamp::now(),
            },
        );
        Ok(AppLock {
            locks: self.clone(),
            key,
            app: app.to_string(),
            operation: operation.to_string(),
            push_to: None,
        })
    }

    /// Like [`Self::try_lock`], but refuses a busy app with a 409 naming the
    /// running operation.
    pub fn lock(&self, app: &str, operation: &str) -> Result<AppLock, AppError> {
        self.try_lock(app, operation)
            .map_err(|holder| AppError::OperationInProgress {
                app: app.to_string(),
                operation: holder,
            })
    }

    /// The operation running on `app`, if any.
    pub fn current(&self, app: &str) -> Option<AppOperation> {
        self.entries().get(&slugify(app)).cloned()
    }
}

/// Holds the operation lock of one app; see [`AppLocks`].
pub struct AppLock {
    locks: AppLocks,
    /// The entry in [`AppLocks`] (the slug of `app`).
    key: String,
    /// The app's name as the caller gave it, which is the name the
    /// authorization service knows the app's scopes by. A directory placed by
    /// hand (e.g. `simple_nginx`) keeps its name, which differs from its slug,
    /// so the release must be announced under this name, not under `key`.
    app: String,
    operation: String,
    /// Where to announce the release, for operations that run as a task.
    push_to: Option<SharedAppState>,
}

impl std::fmt::Debug for AppLock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppLock")
            .field("app", &self.app)
            .field("operation", &self.operation)
            .finish()
    }
}

impl AppLock {
    /// Record the operation's task and announce the release to clients that
    /// may view the app.
    pub fn set_task(&mut self, task_id: Uuid, app_state: SharedAppState) {
        if let Some(entry) = self.locks.entries().get_mut(&self.key) {
            entry.task_id = Some(task_id);
        }
        self.push_to = Some(app_state);
    }

    /// The operation this lock was taken for.
    pub fn operation(&self) -> &str {
        &self.operation
    }
}

impl Drop for AppLock {
    fn drop(&mut self) {
        self.locks.entries().remove(&self.key);
        // Drop can't await; also reached on panic unwind.
        if let (Some(state), Ok(rt)) = (self.push_to.take(), tokio::runtime::Handle::try_current())
        {
            let app = self.app.clone();
            rt.spawn(async move { state.push_app_info(&app).await });
        }
    }
}

pub type SharedAppState = Arc<AppState>;

/// Loads the authorization service from `config_dir`. Any error stops
/// startup: a policy that doesn't load (e.g. an unknown permission) must never
/// be silently replaced by a fallback. A missing `policy.yaml` is not an error;
/// `ConfigManager::load_config` seeds or defaults it.
async fn load_auth_service(config_dir: &str) -> anyhow::Result<AuthorizationService> {
    let service = AuthorizationService::new(config_dir)
        .await
        .with_context(|| format!("Failed to load authorization config from '{config_dir}'"))?;
    info!("Authorization service loaded successfully from config");
    Ok(service)
}

impl AppState {
    /// Tell the WebSocket clients whose user may view `app` that its info
    /// changed; they refetch it.
    pub async fn push_app_info(&self, app: &str) {
        let message = WebSocketMessage::AppInfoUpdated(app.to_string());
        // Auth disabled means every app is visible to everyone; the single
        // decision point for that is `services::apps::list`.
        let everyone = !self.auth_service.is_enabled().await;
        for (client_id, user) in self.messenger.authenticated_clients().await {
            let user_id = AuthorizationService::get_user_id_for_authorization(&user);
            if everyone
                || self
                    .auth_service
                    .decide(&user_id, Permission::View.into(), Target::App(app))
                    .is_allowed()
            {
                let _ = self
                    .messenger
                    .send_to_client(client_id, message.clone())
                    .await;
            }
        }
    }

    /// Records when the next scheduled app-state sweep is due.
    pub fn set_next_app_check(&self, at: jiff::Timestamp) {
        self.next_app_check.store(at.as_second(), Ordering::Relaxed);
    }

    /// The next scheduled app-state sweep, or `None` if no sweep has run yet.
    pub fn next_app_check_at(&self) -> Option<jiff::Timestamp> {
        match self.next_app_check.load(Ordering::Relaxed) {
            0 => None,
            timestamp => jiff::Timestamp::from_second(timestamp).ok(),
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

        let auth_service = Arc::new(load_auth_service("config/casbin").await?);

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
            app_locks: AppLocks::default(),
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
            app_locks: AppLocks::default(),
            session_server: None,
            mcp_server: None,
        }))
    }
}

#[cfg(test)]
mod app_lock_tests {
    use super::AppLocks;

    #[test]
    fn one_operation_per_app_at_a_time() {
        let locks = AppLocks::default();
        let first = locks.try_lock("demo", "rebuild").expect("free");
        let holder = locks.try_lock("demo", "stop").unwrap_err();
        assert_eq!(holder.operation, "rebuild");
        assert_eq!(holder, locks.current("demo").unwrap());
        assert!(
            locks.try_lock("other", "run").is_ok(),
            "other apps are independent"
        );
        drop(first);
        assert!(locks.current("demo").is_none());
        assert!(locks.try_lock("demo", "stop").is_ok(), "released on drop");
    }

    #[test]
    fn keyed_by_directory_name() {
        let locks = AppLocks::default();
        let _lock = locks.try_lock("My App", "create").unwrap();
        assert!(locks.try_lock("my-app", "run").is_err());
    }

    #[tokio::test]
    async fn released_when_the_holding_future_panics() {
        let locks = AppLocks::default();
        let held = locks.clone();
        let result = tokio::spawn(async move {
            let _lock = held.try_lock("demo", "run").unwrap();
            panic!("boom");
        })
        .await;
        assert!(result.is_err());
        assert!(locks.current("demo").is_none());
    }
}

#[cfg(test)]
mod load_auth_service_tests {
    use super::load_auth_service;

    #[tokio::test]
    async fn unknown_permission_stops_startup_naming_role_and_string() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("policy.yaml"),
            "scopes: {}\nroles:\n  viewer:\n    permissions: [veiw]\n    description: d\nassignments: {}\n",
        )
        .unwrap();
        let Err(err) = load_auth_service(dir.path().to_str().unwrap()).await else {
            panic!("a policy with an unknown permission must not load");
        };
        let msg = format!("{err:#}");
        assert!(msg.contains("viewer") && msg.contains("veiw"), "{msg}");
    }
}
