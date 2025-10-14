use std::collections::HashMap;

use bollard::secret::ContainerInspectResponse;
use regex::Regex;

use crate::settings::config::Settings;
use scotty_core::apps::app_data::AppSettings;

use super::types::{
    DockerComposeConfig, DockerComposeServiceConfig, LoadBalancerImpl, LoadBalancerInfo,
};

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
                        "HTTP_AUTH_USER" => {
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
        all_services: &[String],
    ) -> anyhow::Result<DockerComposeConfig> {
        let mut config = DockerComposeConfig {
            services: HashMap::new(),
            networks: None,
        };

        // First, apply environment variables to all services
        if !resolved_environment.is_empty() {
            for service_name in all_services {
                let mut service_config = DockerComposeServiceConfig {
                    labels: None,
                    environment: Some(HashMap::new()),
                    networks: None,
                };
                let environment = service_config.environment.as_mut().unwrap();
                for (key, value) in resolved_environment {
                    environment.insert(key.clone(), value.clone());
                }
                config.services.insert(service_name.clone(), service_config);
            }
        }

        // Then, add load balancer configuration for public services
        for service in &settings.public_services {
            // Get or create the service config (it may already exist from the all_services loop)
            let service_config = config
                .services
                .entry(service.service.clone())
                .or_insert_with(|| DockerComposeServiceConfig {
                    labels: None,
                    environment: None,
                    networks: None,
                });

            // Initialize environment if not present
            if service_config.environment.is_none() {
                service_config.environment = Some(HashMap::new());
            }
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

            // Environment variables are already added in the all_services loop above
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maplit::hashmap;
    use scotty_core::apps::app_data::{AppSettings, ServicePortMapping};
    use scotty_core::settings::loadbalancer::HaproxyConfigSettings;

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
        let all_services = vec!["web".to_string(), "api".to_string()];
        let result = load_balancer
            .get_docker_compose_override(
                &global_settings,
                "myapp",
                &app_settings,
                &app_settings.environment,
                &all_services,
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
        let all_services = vec!["web".to_string()];
        let result = load_balancer
            .get_docker_compose_override(
                &global_settings,
                "myapp",
                &app_settings,
                &app_settings.environment,
                &all_services,
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
    fn test_haproxy_env_vars_applied_to_all_containers() {
        let global_settings = Settings {
            haproxy: HaproxyConfigSettings::new(false),
            ..Default::default()
        };

        let app_settings = AppSettings {
            domain: "example.com".to_string(),
            public_services: vec![ServicePortMapping {
                service: "web".to_string(),
                port: 8080,
                domains: vec![],
            }],
            basic_auth: None,
            disallow_robots: false,
            environment: hashmap! {
                "FOO".to_string() => "BAR".to_string(),
                "DATABASE_URL".to_string() => "postgres://localhost/db".to_string(),
            },
            ..Default::default()
        };

        let load_balancer = HaproxyLoadBalancer;
        // Simulate having multiple services: web (public) and db (not public)
        let all_services = vec!["web".to_string(), "db".to_string(), "redis".to_string()];
        let result = load_balancer
            .get_docker_compose_override(
                &global_settings,
                "myapp",
                &app_settings,
                &app_settings.environment,
                &all_services,
            )
            .unwrap();

        // Check that web service has both environment variables and load balancer config
        let web_config = result.services.get("web").unwrap();
        let web_env = web_config.environment.as_ref().unwrap();
        assert_eq!(web_env.get("FOO").unwrap(), "BAR");
        assert_eq!(
            web_env.get("DATABASE_URL").unwrap(),
            "postgres://localhost/db"
        );
        assert_eq!(web_env.get("VHOST").unwrap(), "web.example.com"); // Has load balancer config
        assert_eq!(web_env.get("VPORT").unwrap(), "8080");

        // Check that db service has environment variables but no load balancer config
        let db_config = result.services.get("db").unwrap();
        let db_env = db_config.environment.as_ref().unwrap();
        assert_eq!(db_env.get("FOO").unwrap(), "BAR");
        assert_eq!(
            db_env.get("DATABASE_URL").unwrap(),
            "postgres://localhost/db"
        );
        assert!(db_env.get("VHOST").is_none()); // No load balancer config
        assert!(db_env.get("VPORT").is_none());

        // Check that redis service has environment variables but no load balancer config
        let redis_config = result.services.get("redis").unwrap();
        let redis_env = redis_config.environment.as_ref().unwrap();
        assert_eq!(redis_env.get("FOO").unwrap(), "BAR");
        assert_eq!(
            redis_env.get("DATABASE_URL").unwrap(),
            "postgres://localhost/db"
        );
        assert!(redis_env.get("VHOST").is_none()); // No load balancer config
        assert!(redis_env.get("VPORT").is_none());
    }
}
