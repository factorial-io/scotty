use std::collections::HashMap;

use serde::Deserialize;

use crate::utils::secret::MaskedSecret;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum DockerConnectOptions {
    Socket,
    Local,
    Http,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct DockerRegistrySettings {
    pub registry: String,
    pub username: String,
    pub password: MaskedSecret,
}
#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct DockerSettings {
    pub connection: DockerConnectOptions,
    pub registries: HashMap<String, DockerRegistrySettings>,
}
