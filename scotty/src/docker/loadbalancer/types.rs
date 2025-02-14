use std::collections::HashMap;

use bollard::secret::ContainerInspectResponse;
use serde::{Deserialize, Serialize};

use crate::settings::config::Settings;
use scotty_core::apps::app_data::AppSettings;

pub struct LoadBalancerInfo {
    pub domains: Vec<String>,
    pub port: Option<u32>,
    pub tls_enabled: bool,
    pub basic_auth_user: Option<String>,
    pub basic_auth_pass: Option<String>,
}

impl Default for LoadBalancerInfo {
    fn default() -> Self {
        LoadBalancerInfo {
            domains: vec![],
            port: Some(80),
            tls_enabled: false,
            basic_auth_user: None,
            basic_auth_pass: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DockerComposeServiceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub networks: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DockerComposeNetworkConfig {
    pub external: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DockerComposeConfig {
    pub services: HashMap<String, DockerComposeServiceConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub networks: Option<HashMap<String, DockerComposeNetworkConfig>>,
}

pub trait LoadBalancerImpl {
    fn get_load_balancer_info(&self, insights: ContainerInspectResponse) -> LoadBalancerInfo;
    fn get_docker_compose_override(
        &self,
        global_settings: &Settings,
        app_name: &str,
        settings: &AppSettings,
        resolved_environment: &HashMap<String, String>,
    ) -> anyhow::Result<DockerComposeConfig>;
}
