use std::{collections::HashMap, sync::Arc};

use bollard::Docker;
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

use crate::apps::shared_app_list::SharedAppList;
use crate::settings::DockerConnectOptions;
use crate::tasks::manager;
use crate::{settings::Settings, stop_flag};

type WebSocketClients = HashMap<Uuid, broadcast::Sender<axum::extract::ws::Message>>;

#[derive(Debug, Clone)]
pub struct AppState {
    pub settings: Settings,
    pub stop_flag: stop_flag::StopFlag,
    pub clients: Arc<Mutex<WebSocketClients>>,
    pub apps: SharedAppList,
    pub docker: Docker,
    pub task_manager: manager::TaskManager,
}

pub type SharedAppState = Arc<AppState>;

impl AppState {
    pub async fn new() -> anyhow::Result<SharedAppState> {
        let settings = Settings::new()?;
        println!("Used settings: {:?}", &settings);

        let stop_flag = stop_flag::StopFlag::new();
        stop_flag::register_signal_handler(&stop_flag);

        let docker = match &settings.docker {
            DockerConnectOptions::Local => Docker::connect_with_local_defaults()?,
            DockerConnectOptions::Socket => Docker::connect_with_socket_defaults()?,
            DockerConnectOptions::Http => Docker::connect_with_http_defaults()?,
        };

        Ok(Arc::new(AppState {
            settings,
            stop_flag: stop_flag.clone(),
            clients: Arc::new(Mutex::new(HashMap::new())),
            apps: SharedAppList::new(),
            docker,
            task_manager: manager::TaskManager::new(),
        }))
    }
}
