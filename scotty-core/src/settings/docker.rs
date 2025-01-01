use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
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
    pub password: String,
}
#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct DockerSettings {
    pub connection: DockerConnectOptions,
    pub registries: HashMap<String, DockerRegistrySettings>,
}
