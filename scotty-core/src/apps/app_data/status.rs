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

/// Derives the overall app status from its containers.
///
/// Completed one-shot/init containers (`Exited` with exit code 0) are tolerated:
/// they do not hold the app at `Starting` once the long-running services are up.
///
/// - `Running`: at least one container is running and every container is either
///   running or completed successfully.
/// - `Stopped`: nothing is running and nothing has completed.
/// - `Starting`: anything else (containers still coming up, or a failed
///   container alongside completed ones but nothing running yet).
pub fn get_app_status_from_services(services: &[ContainerState]) -> AppStatus {
    // Count strictly `Running` containers (as before); `Created`/`Restarting`
    // still read as "coming up", keeping the app at `Starting`.
    let running = count_state(services, ContainerStatus::Running);
    let completed = services.iter().filter(|s| s.is_completed()).count();

    match (running, completed) {
        (0, 0) => AppStatus::Stopped,
        (r, c) if r > 0 && r + c == services.len() => AppStatus::Running,
        (0, _) => AppStatus::Stopped,
        _ => AppStatus::Starting,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svc(status: ContainerStatus, exit_code: Option<i64>) -> ContainerState {
        ContainerState {
            status,
            exit_code,
            ..Default::default()
        }
    }

    #[test]
    fn empty_app_is_stopped() {
        assert_eq!(get_app_status_from_services(&[]), AppStatus::Stopped);
    }

    #[test]
    fn all_running_is_running() {
        let services = vec![
            svc(ContainerStatus::Running, None),
            svc(ContainerStatus::Running, None),
        ];
        assert_eq!(get_app_status_from_services(&services), AppStatus::Running);
    }

    #[test]
    fn running_web_with_completed_init_is_running() {
        // The key edge case: an init container that finished (Exited 0) must
        // not hold the app at Starting while the web container runs.
        let services = vec![
            svc(ContainerStatus::Running, None),
            svc(ContainerStatus::Exited, Some(0)),
        ];
        assert_eq!(get_app_status_from_services(&services), AppStatus::Running);
    }

    #[test]
    fn partially_started_is_starting() {
        let services = vec![
            svc(ContainerStatus::Running, None),
            svc(ContainerStatus::Created, None),
        ];
        assert_eq!(get_app_status_from_services(&services), AppStatus::Starting);
    }

    #[test]
    fn failed_container_with_running_sibling_is_starting() {
        // A crashed container is neither running nor completed, so the app is
        // not fully Running; per-service status still surfaces the failure.
        let services = vec![
            svc(ContainerStatus::Running, None),
            svc(ContainerStatus::Exited, Some(1)),
        ];
        assert_eq!(get_app_status_from_services(&services), AppStatus::Starting);
    }

    #[test]
    fn all_exited_is_stopped() {
        let services = vec![
            svc(ContainerStatus::Exited, Some(0)),
            svc(ContainerStatus::Exited, Some(1)),
        ];
        assert_eq!(get_app_status_from_services(&services), AppStatus::Stopped);
    }

    #[test]
    fn only_completed_one_shots_is_stopped() {
        // Nothing live, only finished one-shots: the app is not Running.
        let services = vec![svc(ContainerStatus::Exited, Some(0))];
        assert_eq!(get_app_status_from_services(&services), AppStatus::Stopped);
    }
}
