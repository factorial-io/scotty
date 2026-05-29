use std::fmt::Display;

use bollard_stubs::models::ContainerStateStatusEnum;
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
    Stopping,
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
            ContainerStatus::Stopping => write!(f, "stopping"),
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
            ContainerStateStatusEnum::STOPPING => ContainerStatus::Stopping,
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

    /// Returns true when the container is in a terminal state and will never
    /// produce more log output, so live `follow` is impossible. Non-terminal
    /// states (running, paused, created, restarting, stopping) may still emit
    /// output — possibly after a state change such as unpausing or starting.
    ///
    /// `Empty` is the default/unknown state (no Docker status record, e.g. a
    /// container that no longer exists); it is treated as terminal so we
    /// conservatively avoid attempting a live follow against something we
    /// can't reason about.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            ContainerStatus::Exited
                | ContainerStatus::Dead
                | ContainerStatus::Removing
                | ContainerStatus::Empty
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    fn state(status: ContainerStatus) -> ContainerState {
        ContainerState {
            status,
            ..Default::default()
        }
    }

    #[test]
    fn terminal_states_are_terminal() {
        for status in [
            ContainerStatus::Exited,
            ContainerStatus::Dead,
            ContainerStatus::Removing,
            ContainerStatus::Empty,
        ] {
            assert!(
                state(status.clone()).is_terminal(),
                "{status} should be terminal"
            );
        }
    }

    #[test]
    fn live_states_are_not_terminal() {
        // These may still produce output (now or after a state change), so they
        // must not be treated as terminal — live follow stays possible.
        for status in [
            ContainerStatus::Running,
            ContainerStatus::Created,
            ContainerStatus::Restarting,
            ContainerStatus::Paused,
            ContainerStatus::Stopping,
        ] {
            assert!(
                !state(status.clone()).is_terminal(),
                "{status} should not be terminal"
            );
        }
    }
}
