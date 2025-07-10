use std::fmt::Display;

use bollard::secret::ContainerStateStatusEnum;
use chrono::TimeDelta;
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse, PartialEq)]
pub enum ContainerStatus {
    Empty,
    Created,
    Restarting,
    Running,
    Paused,
    Removing,
    Exited,
    Dead,
}

impl Display for ContainerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerStatus::Empty => write!(f, "empty"),
            ContainerStatus::Created => write!(f, "created"),
            ContainerStatus::Restarting => write!(f, "restarting"),
            ContainerStatus::Running => write!(f, "running"),
            ContainerStatus::Paused => write!(f, "paused"),
            ContainerStatus::Removing => write!(f, "removing"),
            ContainerStatus::Exited => write!(f, "exited"),
            ContainerStatus::Dead => write!(f, "dead"),
        }
    }
}

impl From<ContainerStateStatusEnum> for ContainerStatus {
    fn from(status: ContainerStateStatusEnum) -> Self {
        match status {
            ContainerStateStatusEnum::EMPTY => ContainerStatus::Empty,
            ContainerStateStatusEnum::CREATED => ContainerStatus::Created,
            ContainerStateStatusEnum::RESTARTING => ContainerStatus::Restarting,
            ContainerStateStatusEnum::RUNNING => ContainerStatus::Running,
            ContainerStateStatusEnum::PAUSED => ContainerStatus::Paused,
            ContainerStateStatusEnum::REMOVING => ContainerStatus::Removing,
            ContainerStateStatusEnum::EXITED => ContainerStatus::Exited,
            ContainerStateStatusEnum::DEAD => ContainerStatus::Dead,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse)]
pub struct ContainerState {
    pub status: ContainerStatus,
    pub id: Option<String>,
    pub service: String,
    pub domains: Vec<String>,
    pub use_tls: bool,
    pub port: Option<u32>,
    pub started_at: Option<chrono::DateTime<chrono::Local>>,
    pub used_registry: Option<String>,
    pub basic_auth: Option<(String, String)>,
}

impl Default for ContainerState {
    fn default() -> Self {
        ContainerState {
            status: ContainerStatus::Empty,
            id: None,
            service: "".to_string(),
            domains: vec![],
            use_tls: false,
            port: None,
            started_at: None,
            used_registry: None,
            basic_auth: None,
        }
    }
}

impl ContainerState {
    pub fn is_running(&self) -> bool {
        self.status == ContainerStatus::Running
            || self.status == ContainerStatus::Created
            || self.status == ContainerStatus::Restarting
    }

    pub fn running_since(&self) -> Option<TimeDelta> {
        self.started_at
            .map(|started_at| chrono::Local::now() - started_at)
    }

    pub fn get_urls(&self) -> Vec<String> {
        self.domains
            .iter()
            .map(|domain| {
                if self.use_tls {
                    format!("https://{domain}")
                } else {
                    format!("http://{domain}")
                }
            })
            .collect()
    }
}
