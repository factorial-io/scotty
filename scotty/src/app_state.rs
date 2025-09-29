use std::{collections::HashMap, sync::Arc};

use bollard::Docker;
use scotty_core::apps::shared_app_list::SharedAppList;
use scotty_core::settings::docker::DockerConnectOptions;
use tokio::sync::{broadcast, Mutex};
use tracing::{info, warn};

use crate::api::basic_auth::CurrentUser;
use crate::api::websocket::WebSocketMessenger;
use crate::docker::services::logs::LogStreamingService;
use crate::oauth::handlers::OAuthState;
use crate::oauth::{
    self, create_device_flow_store, create_oauth_session_store, create_web_flow_store,
};
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
    pub task_output_service: TaskOutputStreamingService,
    pub messenger: WebSocketMessenger,
}

pub type SharedAppState = Arc<AppState>;

impl AppState {
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

        // Initialize OAuth if configured
        let oauth_state = match oauth::client::create_oauth_client(&settings.api.oauth) {
            Ok(Some(client)) => {
                tracing::info!("OAuth client initialized");
                Some(OAuthState {
                    client,
                    device_flow_store: create_device_flow_store(),
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

        // Create WebSocket clients and messenger
        let clients = Arc::new(Mutex::new(HashMap::new()));
        let messenger = WebSocketMessenger::new(clients.clone());

        let state = Arc::new(AppState {
            settings,
            stop_flag: stop_flag.clone(),
            apps: SharedAppList::new(),
            docker,
            task_manager: manager::TaskManager::new(messenger.clone()),
            oauth_state,
            auth_service,
            logs_service,
            task_output_service: TaskOutputStreamingService::new(),
            messenger,
        });

        Ok(state)
    }

    pub async fn new_for_config_only() -> anyhow::Result<SharedAppState> {
        let settings = Settings::new()?;
        let docker = Docker::connect_with_local_defaults()?;

        let clients = Arc::new(Mutex::new(HashMap::new()));
        let messenger = WebSocketMessenger::new(clients.clone());

        Ok(Arc::new(AppState {
            settings,
            stop_flag: stop_flag::StopFlag::new(),
            apps: SharedAppList::new(),
            docker: docker.clone(),
            task_manager: manager::TaskManager::new(messenger.clone()),
            oauth_state: None,
            auth_service: Arc::new(AuthorizationService::create_fallback_service(None).await),
            logs_service: LogStreamingService::new(docker),
            task_output_service: TaskOutputStreamingService::new(),
            messenger,
        }))
    }
}
