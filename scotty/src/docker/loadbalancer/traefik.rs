use std::collections::HashMap;

use bollard::secret::ContainerInspectResponse;
use regex::Regex;

use crate::settings::config::Settings;
use scotty_core::apps::app_data::AppSettings;

use super::types::{
    DockerComposeConfig, DockerComposeNetworkConfig, DockerComposeServiceConfig, LoadBalancerImpl,
    LoadBalancerInfo,
};

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

            // Add custom middlewares from settings
            for middleware in &settings.middlewares {
                middlewares.push(middleware.clone());
            }

            // Connect the middleware to the router
            for (idx, _domain) in domains.iter().enumerate() {
                labels.insert(
                    format!("traefik.http.routers.{}-{}.middlewares", &service_name, idx),
                    middlewares.join(","),
                );
            }

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

#[cfg(test)]
mod tests {
    use super::*;
    use maplit::hashmap;
    use scotty_core::apps::app_data::{AppSettings, ServicePortMapping};
    use scotty_core::settings::loadbalancer::TraefikSettings;

    #[test]
    fn test_traefik_get_docker_compose_override() {
        let global_settings = Settings {
            traefik: TraefikSettings::new(true, "proxy".into(), Some("myresolver".into()), vec![]),
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
            middlewares: vec![
                "custom-middleware-1".to_string(),
                "custom-middleware-2".to_string(),
            ],
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
                .get("traefik.http.routers.web--myapp-0.middlewares")
                .unwrap(),
            "web--myapp--basic-auth,web--myapp--robots,custom-middleware-1,custom-middleware-2"
        );

        // check environment.
        assert_eq!(environment.get("FOO").unwrap(), "BAR");
        assert_eq!(environment.get("API_KEY").unwrap(), "1234");
    }
}
