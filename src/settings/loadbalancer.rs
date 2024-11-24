use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub enum LoadBalancerType {
    HaproxyConfig,
    Traefik,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct TraefikSettings {
    pub use_tls: bool,
    pub network: String,
    pub certresolver: Option<String>,
}

impl TraefikSettings {
    pub fn new(use_tls: bool, network: String, certresolver: Option<String>) -> Self {
        Self {
            use_tls,
            network,
            certresolver,
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
