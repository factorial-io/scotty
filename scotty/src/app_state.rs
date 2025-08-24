use std::{collections::HashMap, sync::Arc};

use bollard::Docker;
use scotty_core::apps::shared_app_list::SharedAppList;
use scotty_core::settings::docker::DockerConnectOptions;
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

use crate::oauth::handlers::OAuthState;
use crate::oauth::{
    self, create_device_flow_store, create_oauth_session_store, create_web_flow_store,
};
use crate::settings::config::Settings;
use crate::stop_flag;
use crate::tasks::manager;

type WebSocketClients = HashMap<Uuid, broadcast::Sender<axum::extract::ws::Message>>;

#[derive(Debug, Clone)]
pub struct AppState {
    pub settings: Settings,
    pub stop_flag: stop_flag::StopFlag,
    pub clients: Arc<Mutex<WebSocketClients>>,
    pub apps: SharedAppList,
    pub docker: Docker,
    pub task_manager: manager::TaskManager,
    pub oauth_state: Option<OAuthState>,
}

pub type SharedAppState = Arc<AppState>;

impl AppState {
    pub async fn new() -> anyhow::Result<SharedAppState> {
        let settings = Settings::new()?;
        println!("Used settings: {:?}", &settings);

        let stop_flag = stop_flag::StopFlag::new();
        stop_flag::register_signal_handler(&stop_flag);

        let docker = match &settings.docker.connection {
            DockerConnectOptions::Local => Docker::connect_with_local_defaults()?,
            DockerConnectOptions::Socket => Docker::connect_with_socket_defaults()?,
            DockerConnectOptions::Http => Docker::connect_with_http_defaults()?,
        };

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

        Ok(Arc::new(AppState {
            settings,
            stop_flag: stop_flag.clone(),
            clients: Arc::new(Mutex::new(HashMap::new())),
            apps: SharedAppList::new(),
            docker,
            task_manager: manager::TaskManager::new(),
            oauth_state,
        }))
    }
}
