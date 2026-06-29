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
}

impl Default for TraefikSettings {
    fn default() -> Self {
        Self {
            use_tls: false,
            network: String::new(),
            certresolver: None,
            allowed_middlewares: Vec::new(),
            container_name: default_traefik_container_name(),
        }
    }
}

impl TraefikSettings {
    pub fn new(
        use_tls: bool,
        network: String,
        certresolver: Option<String>,
        allowed_middlewares: Vec<String>,
    ) -> Self {
        Self {
            use_tls,
            network,
            certresolver,
            allowed_middlewares,
            container_name: default_traefik_container_name(),
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
