use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

use super::container::{ContainerState, ContainerStatus};

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse, PartialEq, Eq)]
pub enum AppStatus {
    Stopped,
    Starting,
    Running,
    Creating,
    Destroying,
    Unsupported,
}

impl std::fmt::Display for AppStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AppStatus::Stopped => write!(f, "Stopped"),
            AppStatus::Starting => write!(f, "Starting"),
            AppStatus::Running => write!(f, "Running"),
            AppStatus::Creating => write!(f, "Creating"),
            AppStatus::Destroying => write!(f, "Destroying"),
            AppStatus::Unsupported => write!(f, "Unsupported"),
        }
    }
}

pub fn count_state(services: &[ContainerState], required: ContainerStatus) -> usize {
    services.iter().filter(|s| s.status == required).count()
}

pub fn get_app_status_from_services(services: &[ContainerState]) -> AppStatus {
    let count_running_services = count_state(services, ContainerStatus::Running);
    match count_running_services {
        0 => AppStatus::Stopped,
        x if x == services.len() => AppStatus::Running,
        _ => AppStatus::Starting,
    }
}
