#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse)]
pub struct ServicePortMapping {
    pub service: String,
    pub port: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse)]
pub struct AppSettings {
    pub needs_setup: bool,
    pub public_services: Vec<ServicePortMapping>,
    pub domain: String,
    pub time_to_live: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            needs_setup: false,
            public_services: Vec::new(),
            domain: "".to_string(),
            time_to_live: 24 * 60 * 60,
        }
    }
}

pub use bollard::models::ContainerStateStatusEnum as ContainerStatus;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse)]
pub struct ContainerState {
    pub status: ContainerStatus,
    pub id: Option<String>,
    pub service: String,
    pub domain: Option<String>,
    pub port: Option<u32>,
    pub started_at: Option<chrono::DateTime<chrono::Local>>,
}

impl Default for ContainerState {
    fn default() -> Self {
        ContainerState {
            status: ContainerStatus::EMPTY,
            id: None,
            service: "".to_string(),
            domain: None,
            port: None,
            started_at: None,
        }
    }
}

impl ContainerState {
    pub fn get_url(&self) -> Option<String> {
        self.domain.as_ref().map(|domain| format!("http://{}", domain))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse)]
pub enum AppState {
    Stopped,
    Starting,
    Running,
}

impl std::fmt::Display for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AppState::Stopped => write!(f, "Stopped"),
            AppState::Starting => write!(f, "Starting"),
            AppState::Running => write!(f, "Running"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse)]
pub struct AppData {
    pub status: AppState,
    pub name: String,
    pub root_directory: String,
    pub docker_compose_path: String,
    pub services: Vec<ContainerState>,
    pub settings: Option<AppSettings>,
}

impl AppData {
    pub fn new(
        name: &str,
        root_directory: &str,
        docker_compose_path: &str,
        services: Vec<ContainerState>,
        settings: Option<AppSettings>,
    ) -> AppData {
        AppData {
            status: get_app_status_from_services(&services),
            name: name.to_string(),
            root_directory: root_directory.to_string(),
            docker_compose_path: docker_compose_path.to_string(),
            services,
            settings,
        }
    }

    pub fn urls(&self) -> Vec<String> {
        self.services.iter().filter_map(|s| s.get_url()).collect()
    }
}

fn count_state(services: &[ContainerState], required: ContainerStatus) -> usize {
    services
        .iter()
        .fold(0, |acc, x| if x.status == required { acc + 1 } else { acc })
}

fn get_app_status_from_services(services: &[ContainerState]) -> AppState {
    let count_running_services = count_state(services, ContainerStatus::RUNNING);
    match count_running_services {
        0 => AppState::Stopped,
        x if x == services.len() => AppState::Running,
        _ => AppState::Starting,
    }
}
