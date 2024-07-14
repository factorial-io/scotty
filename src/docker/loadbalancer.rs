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
}

pub trait GetLoadBalancerInfo {
    fn get_load_balancer_info(&self, insights: ContainerInspectResponse) -> LoadBalancerInfo;
}

pub struct HaproxyLoadBalancer;

impl GetLoadBalancerInfo for HaproxyLoadBalancer {
    fn get_load_balancer_info(&self, _insights: ContainerInspectResponse) -> LoadBalancerInfo {
        // Implement logic to extract LoadBalancerInfo from ContainerState for Haproxy
        LoadBalancerInfo {
            domain: Some("haproxy.example.com".to_string()),
            port: Some(80),
        }
    }
}

pub struct TraefikLoadBalancer;

impl GetLoadBalancerInfo for TraefikLoadBalancer {
    fn get_load_balancer_info(&self, insights: ContainerInspectResponse) -> LoadBalancerInfo {
        let re_host = Regex::new(r"traefik\.http\.routers\.[a-z]*\.rule=Host\(`(.*)`\)").unwrap();
        let re_port =
            Regex::new(r"traefik\.http\.services\.[a-z]*\.loadbalancer.server.port=(.*)").unwrap();
        let mut result = LoadBalancerInfo {
            domain: None,
            port: Some(80),
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
