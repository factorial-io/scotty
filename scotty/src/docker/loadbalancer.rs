use std::collections::HashMap;

use bollard::secret::ContainerInspectResponse;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::settings::config::Settings;
use scotty_core::apps::app_data::AppSettings;
use scotty_core::settings::loadbalancer::LoadBalancerType;

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
                            result.domains = value.split(" ").map(|s| s.to_string()).collect();
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
                            result.basic_auth_user = Some(value.to_string());
                        }
                        "HTTP_AUTH_PASS" => {
                            result.basic_auth_pass = Some(value.to_string());
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
        global_settings: &Settings,
        _app_name: &str,
        settings: &AppSettings,
        resolved_environment: &HashMap<String, String>,
    ) -> anyhow::Result<DockerComposeConfig> {
        let mut config = DockerComposeConfig {
            services: HashMap::new(),
            networks: None,
        };

        for service in &settings.public_services {
            let mut service_config = DockerComposeServiceConfig {
                labels: None,
                environment: Some(HashMap::new()),
                networks: None,
            };
            let environment = service_config.environment.as_mut().unwrap();
            environment.insert(
                "VHOST".into(),
                match &service.domains.is_empty() {
                    false => service.domains.join(" "),
                    true => format!("{}.{}", &service.service, &settings.domain),
                },
            );
            environment.insert("VPORT".into(), format!("{}", &service.port));

            if let Some((basic_auth_user, basic_auth_pass)) = &settings.basic_auth {
                environment.insert("HTTP_AUTH_USER".into(), basic_auth_user.clone());
                environment.insert("HTTP_AUTH_PASS".into(), basic_auth_pass.clone());
            }

            if global_settings.haproxy.use_tls {
                environment.insert("HTTPS_ONLY".into(), "1".into());
            }

            // Handle environment variables
            if !resolved_environment.is_empty() {
                for (key, value) in resolved_environment {
                    environment.insert(key.clone(), value.clone());
                }
            }

            config
                .services
                .insert(service.service.clone(), service_config);
        }

        Ok(config)
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
                        result.domains.push(caps[1].to_string());
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
        global_settings: &Settings,
        app_name: &str,
        settings: &AppSettings,
        resolved_environment: &HashMap<String, String>,
    ) -> anyhow::Result<DockerComposeConfig> {
        let mut config = DockerComposeConfig {
            services: HashMap::new(),
            networks: Some(HashMap::new()),
        };

        // Setup external network with traefik
        let networks = config.networks.as_mut().unwrap();
        networks.insert(
            global_settings.traefik.network.clone(),
            DockerComposeNetworkConfig { external: true },
        );

        for service in &settings.public_services {
            let mut service_config = DockerComposeServiceConfig {
                labels: Some(HashMap::new()),
                environment: Some(HashMap::new()),
                networks: Some(vec![
                    "default".into(),
                    global_settings.traefik.network.clone(),
                ]),
            };
            let labels = service_config.labels.as_mut().unwrap();
            let environment = service_config.environment.as_mut().unwrap();

            let service_name = format!("{}--{}", &service.service, &app_name);

            // Add Traefik labels
            labels.insert("traefik.enable".to_string(), "true".to_string());

            let domains = match &service.domains.is_empty() {
                false => &service.domains,
                true => &vec![format!("{}.{}", &service.service, &settings.domain)],
            };
            for (idx, domain) in domains.iter().enumerate() {
                labels.insert(
                    format!("traefik.http.routers.{}-{}.rule", &service_name, idx),
                    format!("Host(`{}`)", domain),
                );

                if global_settings.traefik.use_tls {
                    labels.insert(
                        format!("traefik.http.routers.{}-{}.tls", &service_name, idx),
                        "true".to_string(),
                    );

                    if let Some(certresolver) = &global_settings.traefik.certresolver {
                        labels.insert(
                            format!(
                                "traefik.http.routers.{}-{}.tls.certresolver",
                                &service_name, idx
                            ),
                            certresolver.clone(),
                        );
                    }
                }
            }

            labels.insert(
                format!(
                    "traefik.http.services.{}.loadbalancer.server.port",
                    &service_name,
                ),
                format!("{}", &service.port),
            );

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

            // Handle environment variables
            if !&resolved_environment.is_empty() {
                for (key, value) in &settings.environment {
                    environment.insert(key.clone(), value.clone());
                }
            }
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
        hashed = hashed.replace('$', "$$")
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
#[cfg(test)]
mod tests {
    use super::*;
    use maplit::hashmap;
    use scotty_core::apps::app_data::{AppSettings, ServicePortMapping};
    use scotty_core::settings::loadbalancer::HaproxyConfigSettings;
    use scotty_core::settings::loadbalancer::TraefikSettings;

    #[test]
    fn test_haproxy_custom_domain_get_docker_compose_override() {
        let global_settings = Settings {
            haproxy: HaproxyConfigSettings::new(false),
            ..Default::default()
        };

        let app_settings = AppSettings {
            domain: "example.com".to_string(),
            public_services: vec![
                ServicePortMapping {
                    service: "web".to_string(),
                    port: 8080,
                    domains: vec!["custom1.test".to_string(), "custom2.test".to_string()],
                },
                ServicePortMapping {
                    service: "api".to_string(),
                    port: 9000,
                    domains: vec!["api1.test".to_string(), "api2.test".to_string()],
                },
            ],
            ..Default::default()
        };

        let load_balancer = HaproxyLoadBalancer;
        let result = load_balancer
            .get_docker_compose_override(
                &global_settings,
                "myapp",
                &app_settings,
                &app_settings.environment,
            )
            .unwrap();

        let web_environment = result
            .services
            .get("web")
            .unwrap()
            .environment
            .as_ref()
            .unwrap();

        assert_eq!(
            web_environment.get("VHOST").unwrap(),
            "custom1.test custom2.test"
        );
        assert_eq!(web_environment.get("VPORT").unwrap(), "8080");
        assert!(web_environment.get("HTTPS_ONLY").is_none());

        let api_config = result.services.get("api").unwrap();
        let api_environment = api_config.environment.as_ref().unwrap();

        assert_eq!(api_environment.get("VHOST").unwrap(), "api1.test api2.test");
        assert_eq!(api_environment.get("VPORT").unwrap(), "9000");
        assert!(api_environment.get("HTTPS_ONLY").is_none());
    }

    #[test]
    fn test_haproxy_config_get_docker_compose_override() {
        let global_settings = Settings {
            haproxy: HaproxyConfigSettings::new(true),
            ..Default::default()
        };

        let app_settings = AppSettings {
            domain: "example.com".to_string(),
            public_services: vec![ServicePortMapping {
                service: "web".to_string(),
                port: 8080,
                domains: vec![],
            }],
            basic_auth: Some(("user".to_string(), "pass".to_string())),
            disallow_robots: true,
            environment: hashmap! {
                "FOO".to_string() => "BAR".to_string(),
                "API_KEY".to_string() => "1234".to_string(),
            },
            ..Default::default()
        };

        let load_balancer = HaproxyLoadBalancer;
        let result = load_balancer
            .get_docker_compose_override(
                &global_settings,
                "myapp",
                &app_settings,
                &app_settings.environment,
            )
            .unwrap();

        let service_config = result.services.get("web").unwrap();
        let environment = service_config.environment.as_ref().unwrap();

        assert_eq!(environment.get("VHOST").unwrap(), "web.example.com");
        assert_eq!(environment.get("VPORT").unwrap(), "8080");
        assert_eq!(environment.get("HTTP_AUTH_USER").unwrap(), "user");
        assert_eq!(environment.get("HTTP_AUTH_PASS").unwrap(), "pass");
        assert_eq!(environment.get("HTTPS_ONLY").unwrap(), "1");
        assert_eq!(environment.get("FOO").unwrap(), "BAR");
        assert_eq!(environment.get("API_KEY").unwrap(), "1234");
    }
    #[test]
    fn test_traefik_get_docker_compose_override() {
        let global_settings = Settings {
            traefik: TraefikSettings::new(true, "proxy".into(), Some("myresolver".into())),
            ..Default::default()
        };

        let app_settings = AppSettings {
            domain: "example.com".to_string(),
            public_services: vec![ServicePortMapping {
                service: "web".to_string(),
                port: 8080,
                domains: vec![],
            }],
            basic_auth: Some(("user".to_string(), "pass".to_string())),
            disallow_robots: true,
            environment: hashmap! {
                "FOO".to_string() => "BAR".to_string(),
                "API_KEY".to_string() => "1234".to_string(),
            },
            ..Default::default()
        };

        let load_balancer = TraefikLoadBalancer;
        let result = load_balancer
            .get_docker_compose_override(
                &global_settings,
                "myapp",
                &app_settings,
                &app_settings.environment,
            )
            .unwrap();

        let service_config = result.services.get("web").unwrap();
        let labels = service_config.labels.as_ref().unwrap();
        let environment = service_config.environment.as_ref().unwrap();

        // check networks.
        let networks = service_config.networks.as_ref().unwrap();
        assert!(networks.contains(&"default".to_string()));
        assert!(networks.contains(&"proxy".to_string()));

        // Check labels.
        assert_eq!(labels.get("traefik.enable").unwrap(), "true");
        assert_eq!(
            labels
                .get("traefik.http.routers.web--myapp-0.rule")
                .unwrap(),
            "Host(`web.example.com`)"
        );
        assert_eq!(
            labels
                .get("traefik.http.services.web--myapp.loadbalancer.server.port")
                .unwrap(),
            "8080"
        );
        assert_eq!(
            labels.get("traefik.http.routers.web--myapp-0.tls").unwrap(),
            "true"
        );
        assert_eq!(
            labels
                .get("traefik.http.routers.web--myapp-0.tls.certresolver")
                .unwrap(),
            "myresolver"
        );
        assert!(
            labels.contains_key("traefik.http.middlewares.web--myapp--basic-auth.basicauth.users")
        );
        assert!(labels.contains_key(
            "traefik.http.middlewares.web--myapp--basic-auth.basicauth.removeheader"
        ));
        assert!(labels.contains_key("traefik.http.middlewares.web--myapp--robots.headers.customresponseheaders.X-Robots-Tags"));
        assert_eq!(
            labels
                .get("traefik.http.routers.web--myapp.middlewares")
                .unwrap(),
            "web--myapp--basic-auth,web--myapp--robots"
        );

        // check environment.
        assert_eq!(environment.get("FOO").unwrap(), "BAR");
        assert_eq!(environment.get("API_KEY").unwrap(), "1234");
    }
}
