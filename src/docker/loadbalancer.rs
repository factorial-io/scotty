use std::collections::HashMap;

use bollard::secret::ContainerInspectResponse;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{apps::app_data::AppSettings, settings::LoadBalancerType};

pub struct LoadBalancerInfo {
    pub domain: Option<String>,
    pub port: Option<u32>,
    pub tls_enabled: bool,
    pub http_auth_user: Option<String>,
    pub http_auth_pass: Option<String>,
}

impl Default for LoadBalancerInfo {
    fn default() -> Self {
        LoadBalancerInfo {
            domain: None,
            port: Some(80),
            tls_enabled: false,
            http_auth_user: None,
            http_auth_pass: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DockerComposeServiceConfig {
    pub labels: Option<HashMap<String, String>>,
    pub environment: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DockerComposeConfig {
    pub services: HashMap<String, DockerComposeServiceConfig>,
}

pub trait LoadBalancerImpl {
    fn get_load_balancer_info(&self, insights: ContainerInspectResponse) -> LoadBalancerInfo;
    fn get_docker_compose_override(
        &self,
        app_name: &str,
        settings: &AppSettings,
    ) -> anyhow::Result<DockerComposeConfig>;
}

pub struct HaproxyLoadBalancer;

impl LoadBalancerImpl for HaproxyLoadBalancer {
    fn get_load_balancer_info(&self, insights: ContainerInspectResponse) -> LoadBalancerInfo {
        let mut result = LoadBalancerInfo {
            ..Default::default()
        };
        if let Some(env_vars) = insights.config.unwrap().env {
            // Define the regular expression to match key-value pairs
            let re = Regex::new(r"^\s*([\w.-]+)\s*=\s*(.*)\s*$").unwrap();

            for var in env_vars {
                if let Some(caps) = re.captures(&var) {
                    let key = caps.get(1).map_or("", |m| m.as_str());
                    let value = caps.get(2).map_or("", |m| m.as_str());

                    match key.to_ascii_uppercase().as_str() {
                        "VHOST" | "VIRTUAL_HOST" => {
                            result.domain = Some(value.to_string());
                        }
                        "VPORT" | "VIRTUAL_PORT" => {
                            if let Ok(port) = value.parse::<u32>() {
                                result.port = Some(port);
                            }
                        }
                        "HTTPS_ONLY" => {
                            if value.to_lowercase() == "true" || value == "1" {
                                result.tls_enabled = true;
                            }
                        }
                        "HTTTP_AUTH_USER" => {
                            result.http_auth_user = Some(value.to_string());
                        }
                        "HTTP_AUTH_PASS" => {
                            result.http_auth_pass = Some(value.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }

        result
    }

    fn get_docker_compose_override(
        &self,
        app_name: &str,
        _settings: &AppSettings,
    ) -> anyhow::Result<DockerComposeConfig> {
        todo!()
    }
}

pub struct TraefikLoadBalancer;

impl LoadBalancerImpl for TraefikLoadBalancer {
    fn get_load_balancer_info(&self, insights: ContainerInspectResponse) -> LoadBalancerInfo {
        let re_host =
            Regex::new(r"traefik\.http\.routers\.[a-z-0-9]*\.rule=Host\(`(.*)`\)").unwrap();
        let re_port =
            Regex::new(r"traefik\.http\.services\.[a-z-0-9]*\.loadbalancer.server.port=(.*)")
                .unwrap();
        let mut result = LoadBalancerInfo {
            ..Default::default()
        };
        if let Some(labels) = insights.config.unwrap().labels {
            // Filter for Traefik labels and find the host
            for (key, value) in labels.iter() {
                let haystack = format!("{}={}", key, value);
                if re_host.is_match(&haystack) {
                    if let Some(caps) = re_host.captures(&haystack) {
                        result.domain = Some(caps[1].to_string());
                    }
                }
                if re_port.is_match(&haystack) {
                    if let Some(caps) = re_port.captures(&haystack) {
                        if let Ok(port) = caps[1].parse::<u32>() {
                            result.port = Some(port);
                        }
                    }
                }
            }
        }

        result
    }

    fn get_docker_compose_override(
        &self,
        app_name: &str,
        settings: &AppSettings,
    ) -> anyhow::Result<DockerComposeConfig> {
        let mut config = DockerComposeConfig {
            services: HashMap::new(),
        };

        for service in &settings.public_services {
            let mut service_config = DockerComposeServiceConfig {
                labels: Some(HashMap::new()),
                environment: Some(HashMap::new()),
            };
            let labels = service_config.labels.as_mut().unwrap();

            let service_name = format!("{}--{}", &service.service, &app_name);

            // Add Traefik labels
            labels.insert("traefik.enable".to_string(), "true".to_string());
            labels.insert(
                format!("traefik.http.routers.{}.rule", &service_name),
                format!("Host(`{}.{}`)", &service.service, &settings.domain),
            );
            labels.insert(
                format!(
                    "traefik.http.services.{}.loadbalancer.server.port",
                    &service_name,
                ),
                format!("{}", &service.port),
            );
            if settings.use_tls {
                labels.insert(
                    format!("traefik.http.routers.{}.tls", &service_name),
                    "true".to_string(),
                );
            }

            let mut middlewares = vec![];

            if let Some((basic_auth_user, basic_auth_pass)) = &settings.basic_auth {
                let middleware_name = format!("{}--{}", &service_name, "basic-auth");
                labels.insert(
                    format!(
                        "traefik.http.middlewares.{}.basicauth.users",
                        &middleware_name
                    ),
                    format!("{}:{}", basic_auth_user, htpasswd(basic_auth_pass, true)?),
                );
                labels.insert(
                    format!(
                        "traefik.http.middlewares.{}.basicauth.removeheader",
                        &middleware_name
                    ),
                    "true".to_string(),
                );

                middlewares.push(middleware_name.clone());
            }

            if settings.disallow_robots {
                let middleware_name = format!("{}--{}", &service_name, "robots");
                labels.insert(
                    format!(
                        "traefik.http.middlewares.{}.headers.customresponseheaders.X-Robots-Tags",
                        &middleware_name
                    ),
                    "none, noarchive, nosnippet, notranslate, noimageindex".to_string(),
                );

                middlewares.push(middleware_name.clone());
            }
            // Connect the middleware to the router
            labels.insert(
                format!("traefik.http.routers.{}.middlewares", &service_name,),
                middlewares.join(","),
            );
            config
                .services
                .insert(service.service.clone(), service_config);
        }

        Ok(config)
    }
}

fn htpasswd(password: &str, escape_dollars: bool) -> anyhow::Result<String> {
    use bcrypt::{hash, DEFAULT_COST};

    let mut hashed = hash(password, DEFAULT_COST)?;
    if escape_dollars {
        hashed = hashed.replace("$", "$$")
    }
    Ok(hashed)
}

pub struct LoadBalancerFactory;

impl LoadBalancerFactory {
    pub fn create(load_balancer_type: &LoadBalancerType) -> Box<dyn LoadBalancerImpl> {
        match load_balancer_type {
            LoadBalancerType::HaproxyConfig => Box::new(HaproxyLoadBalancer),
            LoadBalancerType::Traefik => Box::new(TraefikLoadBalancer),
        }
    }
}
