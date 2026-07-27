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
    /// Exit code reported by Docker for a stopped container, when available.
    /// Used to tell a clean one-shot/init exit (`Exited` + `Some(0)`) apart
    /// from a crash (non-zero exit code).
    pub exit_code: Option<i64>,
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
            exit_code: None,
        }
    }
}

impl ContainerState {
    pub fn is_running(&self) -> bool {
        self.status == ContainerStatus::Running
            || self.status == ContainerStatus::Created
            || self.status == ContainerStatus::Restarting
    }

    /// Returns true when the container ran to completion successfully, i.e. it
    /// has `Exited` with exit code `0`. This is the typical one-shot/init
    /// container case: the work is done and should not be treated as a failure
    /// when computing app-level status. A non-zero exit code is a crash, not a
    /// completion.
    pub fn is_completed(&self) -> bool {
        self.status == ContainerStatus::Exited && self.exit_code == Some(0)
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

    fn exited(exit_code: Option<i64>) -> ContainerState {
        ContainerState {
            status: ContainerStatus::Exited,
            exit_code,
            ..Default::default()
        }
    }

    #[test]
    fn running_states_are_running() {
        for status in [
            ContainerStatus::Running,
            ContainerStatus::Created,
            ContainerStatus::Restarting,
        ] {
            assert!(
                state(status.clone()).is_running(),
                "{status} should be running"
            );
        }
    }

    #[test]
    fn non_running_states_are_not_running() {
        for status in [
            ContainerStatus::Exited,
            ContainerStatus::Dead,
            ContainerStatus::Paused,
            ContainerStatus::Stopping,
            ContainerStatus::Removing,
            ContainerStatus::Empty,
        ] {
            assert!(
                !state(status.clone()).is_running(),
                "{status} should not be running"
            );
        }
    }

    #[test]
    fn exited_zero_is_completed() {
        assert!(exited(Some(0)).is_completed());
    }

    #[test]
    fn exited_non_zero_is_not_completed() {
        assert!(!exited(Some(1)).is_completed());
        assert!(!exited(Some(137)).is_completed());
    }

    #[test]
    fn exited_without_exit_code_is_not_completed() {
        // No exit code available: do not assume success.
        assert!(!exited(None).is_completed());
    }

    #[test]
    fn running_states_are_not_completed() {
        // A clean exit code only matters for an Exited container.
        for status in [
            ContainerStatus::Running,
            ContainerStatus::Created,
            ContainerStatus::Restarting,
            ContainerStatus::Dead,
        ] {
            let s = ContainerState {
                status: status.clone(),
                exit_code: Some(0),
                ..Default::default()
            };
            assert!(!s.is_completed(), "{status} should not be completed");
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
