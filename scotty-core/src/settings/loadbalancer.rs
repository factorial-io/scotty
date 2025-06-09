#![allow(dead_code)]

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum LoadBalancerType {
    HaproxyConfig,
    Traefik,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct TraefikSettings {
    pub use_tls: bool,
    pub network: String,
    pub certresolver: Option<String>,
    #[serde(default)]
    pub allowed_middlewares: Vec<String>,
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
