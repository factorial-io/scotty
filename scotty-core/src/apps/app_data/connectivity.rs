use std::fmt::Display;

use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

/// Whether the load balancer can actually reach an app.
///
/// Traefik's membership in an app's per-app proxy network lives in container
/// state, not in declared config, so it can be lost without anything about the
/// app itself changing (e.g. the Traefik container is recreated). This state is
/// written by the proxy-network reconciler from what it observed in Docker, and
/// is deliberately *not* derived from the app's own status: an app whose
/// containers are all `Running` can still be unreachable.
#[derive(
    Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema, ToResponse,
)]
pub enum LoadBalancerConnectivity {
    /// No reconciliation pass has observed this app yet. This is the default so
    /// that every code path constructing `AppData` without consulting Docker
    /// reports "not yet determined" instead of guessing.
    #[default]
    Unknown,
    /// Connectivity does not apply: the app has no public services, the load
    /// balancer is not Traefik, or the app still routes over the legacy shared
    /// proxy network instead of a per-app one.
    NotApplicable,
    /// The load balancer is attached to the app's proxy network.
    Connected,
    /// The app's proxy network exists but the load balancer is not attached to
    /// it, so requests to the app's domains never reach a backend.
    Disconnected,
    /// The load balancer container itself could not be found or inspected, so
    /// no app can be routable.
    LoadBalancerUnavailable,
}

impl LoadBalancerConnectivity {
    /// Returns true when the app is known to be unreachable by the load
    /// balancer. `Unknown` and `NotApplicable` are not problems, so they are
    /// not counted.
    pub fn is_problem(&self) -> bool {
        matches!(
            self,
            LoadBalancerConnectivity::Disconnected
                | LoadBalancerConnectivity::LoadBalancerUnavailable
        )
    }
}

impl Display for LoadBalancerConnectivity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadBalancerConnectivity::Unknown => write!(f, "unknown"),
            LoadBalancerConnectivity::NotApplicable => write!(f, "not applicable"),
            LoadBalancerConnectivity::Connected => write!(f, "connected"),
            LoadBalancerConnectivity::Disconnected => write!(f, "not connected"),
            LoadBalancerConnectivity::LoadBalancerUnavailable => {
                write!(f, "load balancer unavailable")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::app_data::AppData;

    /// A newer client talking to a server that predates the field must not fail
    /// to parse. `scottyctl` is installed independently of the server and
    /// preflight only gates major.minor, so both directions have to work.
    #[test]
    fn payload_without_the_field_deserializes_to_unknown() {
        let payload = serde_json::json!({
            "status": "Running",
            "name": "blog",
            "root_directory": "/apps/blog",
            "docker_compose_path": "/apps/blog/docker-compose.yml",
            "services": [],
            "settings": null,
            "last_checked": null
        });

        let app: AppData = serde_json::from_value(payload).expect("should parse without the field");
        assert_eq!(
            app.load_balancer_connectivity,
            LoadBalancerConnectivity::Unknown
        );
    }

    /// An older client ignores fields it does not know, which serde does by
    /// default as long as nothing denies unknown fields.
    #[test]
    fn unknown_fields_are_ignored_when_parsing() {
        let payload = serde_json::json!({
            "status": "Running",
            "name": "blog",
            "root_directory": "/apps/blog",
            "docker_compose_path": "/apps/blog/docker-compose.yml",
            "services": [],
            "settings": null,
            "last_checked": null,
            "load_balancer_connectivity": "Connected",
            "some_field_from_a_newer_server": 42
        });

        let app: AppData = serde_json::from_value(payload).expect("should tolerate extra fields");
        assert_eq!(
            app.load_balancer_connectivity,
            LoadBalancerConnectivity::Connected
        );
    }

    #[test]
    fn round_trips_through_json() {
        for state in [
            LoadBalancerConnectivity::Unknown,
            LoadBalancerConnectivity::NotApplicable,
            LoadBalancerConnectivity::Connected,
            LoadBalancerConnectivity::Disconnected,
            LoadBalancerConnectivity::LoadBalancerUnavailable,
        ] {
            let json = serde_json::to_string(&state).unwrap();
            let parsed: LoadBalancerConnectivity = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, state);
        }
    }
}
