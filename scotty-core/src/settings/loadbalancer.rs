#![allow(dead_code)]

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum LoadBalancerType {
    HaproxyConfig,
    Traefik,
}

/// Default name of the Traefik container that Scotty connects to each
/// per-app proxy network. Used both as the serde default and the `Default` impl.
pub fn default_traefik_container_name() -> String {
    "traefik".to_string()
}

/// Default base network name. This is the base for each app's per-app proxy
/// network (`<network>--<app>`); it must never be empty.
pub fn default_traefik_network() -> String {
    "proxy".to_string()
}

/// Watch Docker events for the Traefik container starting, by default.
///
/// Enabled by default because the failure it prevents — a recreated Traefik
/// that is no longer attached to any per-app proxy network — is a silent
/// outage. Disabling it only widens the repair window to one
/// `scheduler.running_app_check` interval; it never disables repair.
pub fn default_traefik_watch_docker_events() -> bool {
    true
}

#[derive(Debug, Deserialize, Clone)]
pub struct TraefikSettings {
    pub use_tls: bool,
    pub network: String,
    pub certresolver: Option<String>,
    #[serde(default)]
    pub allowed_middlewares: Vec<String>,
    /// Name (or id) of the running Traefik container. Scotty connects this
    /// container to each app's dedicated proxy network so it can route to the
    /// app's public services without sharing a single global network.
    #[serde(default = "default_traefik_container_name")]
    pub container_name: String,
    /// Watch Docker container events and reconcile the per-app proxy networks
    /// as soon as the Traefik container starts, instead of waiting for the next
    /// scheduled running-app check. The periodic check reconciles regardless;
    /// this only shortens the repair window.
    #[serde(default = "default_traefik_watch_docker_events")]
    pub watch_docker_events: bool,
}

impl Default for TraefikSettings {
    fn default() -> Self {
        Self {
            use_tls: false,
            // Matches the documented default and what config.rs sets. Must be
            // non-empty: it is the base for each app's proxy network
            // (`<network>--<app>`), and an empty base would yield a Docker-invalid
            // name beginning with `-`.
            network: default_traefik_network(),
            certresolver: None,
            allowed_middlewares: Vec::new(),
            container_name: default_traefik_container_name(),
            watch_docker_events: default_traefik_watch_docker_events(),
        }
    }
}

impl TraefikSettings {
    pub fn new(
        use_tls: bool,
        network: String,
        certresolver: Option<String>,
        allowed_middlewares: Vec<String>,
        container_name: String,
    ) -> Self {
        Self {
            use_tls,
            network,
            certresolver,
            allowed_middlewares,
            container_name,
            watch_docker_events: default_traefik_watch_docker_events(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct HaproxyConfigSettings {
    pub use_tls: bool,
}

impl HaproxyConfigSettings {
    pub fn new(use_tls: bool) -> Self {
        Self { use_tls }
    }
}
