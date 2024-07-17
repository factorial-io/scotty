use bollard::secret::ContainerInspectResponse;
use regex::Regex;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub enum LoadBalancerType {
    HaproxyConfig,
    Traefik,
}

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

pub trait GetLoadBalancerInfo {
    fn get_load_balancer_info(&self, insights: ContainerInspectResponse) -> LoadBalancerInfo;
}

pub struct HaproxyLoadBalancer;

impl GetLoadBalancerInfo for HaproxyLoadBalancer {
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
}

pub struct TraefikLoadBalancer;

impl GetLoadBalancerInfo for TraefikLoadBalancer {
    fn get_load_balancer_info(&self, insights: ContainerInspectResponse) -> LoadBalancerInfo {
        let re_host = Regex::new(r"traefik\.http\.routers\.[a-z]*\.rule=Host\(`(.*)`\)").unwrap();
        let re_port =
            Regex::new(r"traefik\.http\.services\.[a-z]*\.loadbalancer.server.port=(.*)").unwrap();
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
}

pub struct LoadBalancerFactory;

impl LoadBalancerFactory {
    pub fn create(load_balancer_type: &LoadBalancerType) -> Box<dyn GetLoadBalancerInfo> {
        match load_balancer_type {
            LoadBalancerType::HaproxyConfig => Box::new(HaproxyLoadBalancer),
            LoadBalancerType::Traefik => Box::new(TraefikLoadBalancer),
        }
    }
}
