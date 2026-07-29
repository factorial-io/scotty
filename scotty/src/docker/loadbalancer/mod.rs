pub mod factory;
pub mod haproxy;
pub mod network_reconciler;
pub mod traefik;
pub mod types;

use bollard::errors::Error as BollardError;
use scotty_core::settings::loadbalancer::LoadBalancerType;

use crate::settings::config::Settings;

/// Where per-app proxy networks live and which container has to be attached to
/// them.
///
/// Resolved from settings in one place so that the deploy-time network handlers
/// and the reconciler can never disagree about the base network name or the
/// Traefik container they operate on.
#[derive(Debug, Clone)]
pub struct TraefikTarget {
    /// Base network name; per-app networks are `<base_network>--<app>`.
    pub base_network: String,
    /// Name (or id) of the Traefik container to attach.
    pub container: String,
}

impl TraefikTarget {
    /// Network name for one app under this target.
    pub fn network_for(&self, app_name: &str) -> String {
        app_proxy_network_name(&self.base_network, app_name)
    }
}

/// Resolves the Traefik target, or `None` when load balancing is not Traefik
/// (e.g. HAProxy), in which case all per-app proxy network handling is a no-op.
pub fn traefik_target(settings: &Settings) -> Option<TraefikTarget> {
    if settings.load_balancer_type != LoadBalancerType::Traefik {
        return None;
    }
    Some(TraefikTarget {
        base_network: settings.traefik.network.clone(),
        container: settings.traefik.container_name.clone(),
    })
}

/// Returns the HTTP status code for a Docker daemon error, if any.
///
/// Callers use this to tolerate the benign "already in that state" responses
/// (403/404/409) that make the network operations idempotent. Which of those a
/// daemon returns is version-dependent, so they are matched as a set rather
/// than individually.
pub fn server_status(err: &BollardError) -> Option<u16> {
    match err {
        BollardError::DockerResponseServerError { status_code, .. } => Some(*status_code),
        _ => None,
    }
}

/// Computes the name of the per-app Traefik proxy network.
///
/// Each app gets its own dedicated external network (derived from the
/// configured base network name plus the app name) instead of all apps
/// sharing one global network. This keeps each app's Docker DNS namespace
/// isolated so service names (e.g. `nginx`) can never collide across apps.
///
/// Must stay in sync between the compose-override generation
/// (`traefik.rs`) and the network lifecycle handlers, which both build the
/// name from the same inputs.
///
/// `app_name` is expected to be a slug (Scotty slugifies app names on
/// create/adopt), which keeps the result within Docker's allowed network-name
/// character set. The join is not injective if `base_network` itself contains
/// `--` (e.g. base `proxy--region` + app `foo` yields the same name as base
/// `proxy` + app `region--foo`); the default base `proxy` has no dashes, so in
/// practice the network name is determined by the app name.
pub fn app_proxy_network_name(base_network: &str, app_name: &str) -> String {
    format!("{base_network}--{app_name}")
}
