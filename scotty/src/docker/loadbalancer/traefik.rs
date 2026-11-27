use std::collections::HashMap;

use bollard_stubs::models::ContainerInspectResponse;
use regex::Regex;

use crate::settings::config::Settings;
use scotty_core::apps::app_data::AppSettings;

use super::types::{
    DockerComposeConfig, DockerComposeNetworkConfig, DockerComposeServiceConfig, LoadBalancerImpl,
    LoadBalancerInfo, ServiceNetworkAttachment,
};
use super::{app_proxy_network_name, ROBOTS_HEADER_VALUE};

pub struct TraefikLoadBalancer;

/// No client has this address, so an `ipallowlist` with it refuses everyone (403).
const BLOCK_ALL_SOURCE_RANGE: &str = "255.255.255.255/32";
/// Far above any rule-length default priority and the default backend.
const BLOCKED_ROUTER_PRIORITY: u32 = 10000;

impl LoadBalancerImpl for TraefikLoadBalancer {
    fn get_load_balancer_info(&self, insights: ContainerInspectResponse) -> LoadBalancerInfo {
        let re_host =
            Regex::new(r"^traefik\.http\.routers\.[a-z0-9-]*\.rule=Host\(`([^`]*)`\)$").unwrap();
        let re_port =
            Regex::new(r"traefik\.http\.services\.[a-z-0-9]*\.loadbalancer.server.port=(.*)")
                .unwrap();
        let mut result = LoadBalancerInfo {
            ..Default::default()
        };
        if let Some(labels) = insights.config.unwrap().labels {
            // Filter for Traefik labels and find the host
            for (key, value) in labels.iter() {
                let haystack = format!("{key}={value}");
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
        all_services: &[String],
    ) -> anyhow::Result<DockerComposeConfig> {
        let mut config = DockerComposeConfig {
            services: HashMap::new(),
            networks: Some(HashMap::new()),
        };

        // Each app gets its own dedicated external proxy network instead of a
        // single shared one, so Docker DNS names never collide across apps.
        // Scotty creates this network and connects Traefik to it via the
        // network lifecycle handlers; here we only reference it as external.
        let app_network = app_proxy_network_name(&global_settings.traefik.network, app_name);
        let networks = config.networks.as_mut().unwrap();
        networks.insert(
            app_network.clone(),
            DockerComposeNetworkConfig {
                external: true,
                name: Some(app_network.clone()),
            },
        );

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

        let blocked_paths = global_settings
            .apps
            .blocked_paths_for(settings.app_blueprint.as_deref());

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

            // Initialize labels if not present
            if service_config.labels.is_none() {
                service_config.labels = Some(HashMap::new());
            }
            // Initialize environment if not present
            if service_config.environment.is_none() {
                service_config.environment = Some(HashMap::new());
            }
            let service_name = format!("{}--{}", service.service, app_name);

            // Attach the public service to its project `default` network and to
            // the per-app proxy network. On the proxy network we set an explicit
            // app-scoped alias so the service is reachable under a unique name
            // (Compose still adds the bare service name as an alias, but the
            // network itself is no longer shared, so that no longer collides).
            let mut service_networks = std::collections::HashMap::new();
            service_networks.insert("default".to_string(), ServiceNetworkAttachment::default());
            service_networks.insert(
                app_network.clone(),
                ServiceNetworkAttachment {
                    aliases: Some(vec![service_name.clone()]),
                },
            );
            service_config.networks = Some(service_networks);

            let labels = service_config.labels.as_mut().unwrap();

            // Add Traefik labels
            labels.insert("traefik.enable".to_string(), "true".to_string());

            // Tell Traefik which network to use to reach this container. The
            // container is on multiple networks and Traefik only resolves the
            // backend IP once at discovery time, so this label is required for
            // correct routing.
            labels.insert("traefik.docker.network".to_string(), app_network.clone());

            // Blocked paths: a second, higher-priority router per domain whose
            // only middleware is an allowlist no client matches, so Traefik
            // answers 403 before the request reaches the app or basic auth.
            let blocked_middleware = format!("{}--blocked", service_name);
            let blocked_rule = blocked_paths
                .iter()
                .map(|p| format!("PathPrefix(`{p}`)"))
                .collect::<Vec<_>>()
                .join(" || ");
            if !blocked_paths.is_empty() {
                labels.insert(
                    format!(
                        "traefik.http.middlewares.{}.ipallowlist.sourcerange",
                        blocked_middleware
                    ),
                    BLOCK_ALL_SOURCE_RANGE.to_string(),
                );
            }

            let domains = service.get_domains(&settings.domain);
            for (idx, domain) in domains.iter().enumerate() {
                let router = format!("{}-{}", service_name, idx);
                let blocked_router = format!("{}-blocked", router);
                let mut routers = vec![&router];

                labels.insert(
                    format!("traefik.http.routers.{}.rule", router),
                    format!("Host(`{domain}`)"),
                );

                if !blocked_paths.is_empty() {
                    routers.push(&blocked_router);
                    labels.insert(
                        format!("traefik.http.routers.{}.rule", blocked_router),
                        format!("Host(`{domain}`) && ({blocked_rule})"),
                    );
                    labels.insert(
                        format!("traefik.http.routers.{}.priority", blocked_router),
                        BLOCKED_ROUTER_PRIORITY.to_string(),
                    );
                    labels.insert(
                        format!("traefik.http.routers.{}.service", blocked_router),
                        service_name.clone(),
                    );
                    labels.insert(
                        format!("traefik.http.routers.{}.middlewares", blocked_router),
                        blocked_middleware.clone(),
                    );
                }

                if global_settings.traefik.use_tls {
                    for r in &routers {
                        labels.insert(
                            format!("traefik.http.routers.{}.tls", r),
                            "true".to_string(),
                        );
                        if let Some(certresolver) = &global_settings.traefik.certresolver {
                            labels.insert(
                                format!("traefik.http.routers.{}.tls.certresolver", r),
                                certresolver.clone(),
                            );
                        }
                    }
                }
            }

            labels.insert(
                format!(
                    "traefik.http.services.{}.loadbalancer.server.port",
                    service_name,
                ),
                format!("{}", service.port),
            );

            let mut middlewares = vec![];

            if let Some((basic_auth_user, basic_auth_pass)) = &settings.basic_auth {
                let middleware_name = format!("{}--{}", service_name, "basic-auth");
                labels.insert(
                    format!(
                        "traefik.http.middlewares.{}.basicauth.users",
                        middleware_name
                    ),
                    format!("{}:{}", basic_auth_user, htpasswd(basic_auth_pass, true)?),
                );
                labels.insert(
                    format!(
                        "traefik.http.middlewares.{}.basicauth.removeheader",
                        middleware_name
                    ),
                    "true".to_string(),
                );

                middlewares.push(middleware_name.clone());
            }

            if settings.disallow_robots {
                let middleware_name = format!("{}--{}", service_name, "robots");
                labels.insert(
                    format!(
                        "traefik.http.middlewares.{}.headers.customresponseheaders.X-Robots-Tag",
                        middleware_name
                    ),
                    ROBOTS_HEADER_VALUE.to_string(),
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
                    format!("traefik.http.routers.{}-{}.middlewares", service_name, idx),
                    middlewares.join(","),
                );
            }

            // Environment variables are already added in the all_services loop above
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
    use scotty_core::utils::secret::SecretHashMap;

    #[test]
    fn test_traefik_get_docker_compose_override() {
        let global_settings = Settings {
            traefik: TraefikSettings::new(
                true,
                "proxy".into(),
                Some("myresolver".into()),
                vec![],
                "traefik".into(),
            ),
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
            environment: SecretHashMap::from_hashmap(hashmap! {
                "FOO".to_string() => "BAR".to_string(),
                "API_KEY".to_string() => "1234".to_string(),
            }),
            middlewares: vec![
                "custom-middleware-1".to_string(),
                "custom-middleware-2".to_string(),
            ],
            ..Default::default()
        };

        let load_balancer = TraefikLoadBalancer;
        let all_services = vec!["web".to_string()];
        let exposed_env = app_settings.environment.expose_all();
        let result = load_balancer
            .get_docker_compose_override(
                &global_settings,
                "myapp",
                &app_settings,
                &exposed_env,
                &all_services,
            )
            .unwrap();

        let service_config = result.services.get("web").unwrap();
        let labels = service_config.labels.as_ref().unwrap();
        let environment = service_config.environment.as_ref().unwrap();

        // check networks: the service joins its `default` network and the
        // per-app proxy network (base "proxy" + app "myapp"), with an
        // app-scoped alias on the proxy network.
        let networks = service_config.networks.as_ref().unwrap();
        assert!(networks.contains_key("default"));
        assert!(networks.contains_key("proxy--myapp"));
        assert_eq!(
            networks
                .get("proxy--myapp")
                .unwrap()
                .aliases
                .as_ref()
                .unwrap(),
            &vec!["web--myapp".to_string()]
        );

        // The top-level network definition is the per-app external network.
        let defined_networks = result.networks.as_ref().unwrap();
        let net = defined_networks.get("proxy--myapp").unwrap();
        assert!(net.external);
        assert_eq!(net.name.as_deref(), Some("proxy--myapp"));

        // Check labels.
        assert_eq!(labels.get("traefik.enable").unwrap(), "true");
        assert_eq!(
            labels.get("traefik.docker.network").unwrap(),
            "proxy--myapp"
        );
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
        assert_eq!(
            labels.get(
                "traefik.http.middlewares.web--myapp--robots.headers.customresponseheaders.X-Robots-Tag"
            ),
            Some(&ROBOTS_HEADER_VALUE.to_string())
        );
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

    #[test]
    fn test_traefik_override_serializes_to_valid_compose() {
        let global_settings = Settings {
            traefik: TraefikSettings::new(false, "proxy".into(), None, vec![], "traefik".into()),
            ..Default::default()
        };
        let app_settings = AppSettings {
            domain: "example.com".to_string(),
            public_services: vec![ServicePortMapping {
                service: "nginx".to_string(),
                port: 80,
                domains: vec![],
            }],
            ..Default::default()
        };

        let result = TraefikLoadBalancer
            .get_docker_compose_override(
                &global_settings,
                "stiftung",
                &app_settings,
                &app_settings.environment.expose_all(),
                &["nginx".to_string()],
            )
            .unwrap();

        let yaml = serde_norway::to_string(&result).unwrap();

        // The serialized override must round-trip as valid YAML and carry the
        // per-app network with an app-scoped alias (not the bare service name).
        let parsed: serde_norway::Value = serde_norway::from_str(&yaml).unwrap();
        let net = &parsed["services"]["nginx"]["networks"];
        assert!(
            net.get("default").is_some(),
            "missing default network in:\n{yaml}"
        );
        // Pin the empty-attachment shape: `default: {}` (a mapping), not `null`.
        assert!(
            net["default"].is_mapping(),
            "expected `default` to serialize as a mapping, got: {:?}\n{yaml}",
            net.get("default")
        );
        let aliases = &parsed["services"]["nginx"]["networks"]["proxy--stiftung"]["aliases"];
        assert_eq!(aliases[0].as_str(), Some("nginx--stiftung"));
        assert_eq!(
            parsed["networks"]["proxy--stiftung"]["external"].as_bool(),
            Some(true)
        );
    }

    #[test]
    fn test_traefik_env_vars_applied_to_all_containers() {
        let global_settings = Settings {
            traefik: TraefikSettings::new(
                true,
                "proxy".into(),
                Some("myresolver".into()),
                vec![],
                "traefik".into(),
            ),
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
            environment: SecretHashMap::from_hashmap(hashmap! {
                "FOO".to_string() => "BAR".to_string(),
                "DATABASE_URL".to_string() => "postgres://localhost/db".to_string(),
            }),
            middlewares: vec![],
            ..Default::default()
        };

        let load_balancer = TraefikLoadBalancer;
        // Simulate having multiple services: web (public) and db (not public)
        let all_services = vec!["web".to_string(), "db".to_string(), "redis".to_string()];
        let exposed_env = app_settings.environment.expose_all();
        let result = load_balancer
            .get_docker_compose_override(
                &global_settings,
                "myapp",
                &app_settings,
                &exposed_env,
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
        assert!(web_config.labels.is_some()); // Has load balancer labels
        assert!(web_config.networks.is_some()); // Has networks

        // With disallow_robots: false, the robots header must not be emitted
        let web_labels = web_config.labels.as_ref().unwrap();
        assert!(!web_labels.contains_key(
            "traefik.http.middlewares.web--myapp--robots.headers.customresponseheaders.X-Robots-Tag"
        ));
        // ...and the robots middleware must not be wired into the router chain
        assert!(!web_labels
            .get("traefik.http.routers.web--myapp-0.middlewares")
            .map(|s| s.contains("web--myapp--robots"))
            .unwrap_or(false));

        // Check that db service has environment variables but no load balancer config
        let db_config = result.services.get("db").unwrap();
        let db_env = db_config.environment.as_ref().unwrap();
        assert_eq!(db_env.get("FOO").unwrap(), "BAR");
        assert_eq!(
            db_env.get("DATABASE_URL").unwrap(),
            "postgres://localhost/db"
        );
        assert!(db_config.labels.is_none()); // No load balancer labels
        assert!(db_config.networks.is_none()); // No networks

        // Check that redis service has environment variables but no load balancer config
        let redis_config = result.services.get("redis").unwrap();
        let redis_env = redis_config.environment.as_ref().unwrap();
        assert_eq!(redis_env.get("FOO").unwrap(), "BAR");
        assert_eq!(
            redis_env.get("DATABASE_URL").unwrap(),
            "postgres://localhost/db"
        );
        assert!(redis_config.labels.is_none()); // No load balancer labels
        assert!(redis_config.networks.is_none()); // No networks
    }

    fn blocked_override(use_tls: bool, global: &[&str]) -> HashMap<String, String> {
        let mut global_settings = Settings {
            traefik: TraefikSettings::new(
                use_tls,
                "proxy".into(),
                Some("myresolver".into()),
                vec![],
                "traefik".into(),
            ),
            ..Default::default()
        };
        global_settings.apps.blocked_paths = global.iter().map(|s| s.to_string()).collect();
        let app_settings = AppSettings {
            domain: "example.com".to_string(),
            public_services: vec![ServicePortMapping {
                service: "web".to_string(),
                port: 80,
                domains: vec!["a.org".to_string(), "b.org".to_string()],
            }],
            basic_auth: Some(("user".to_string(), "pass".to_string())),
            ..Default::default()
        };
        TraefikLoadBalancer
            .get_docker_compose_override(
                &global_settings,
                "myapp",
                &app_settings,
                &HashMap::new(),
                &["web".to_string()],
            )
            .unwrap()
            .services
            .remove("web")
            .unwrap()
            .labels
            .unwrap()
    }

    #[test]
    fn test_traefik_blocked_paths_add_deny_router_per_domain() {
        let labels = blocked_override(true, &["/core/install.php", "/.git/"]);
        let domains = ["a.org", "b.org"];
        assert_eq!(
            labels
                .get("traefik.http.middlewares.web--myapp--blocked.ipallowlist.sourcerange")
                .unwrap(),
            "255.255.255.255/32"
        );
        for (idx, domain) in domains.iter().enumerate() {
            let r = format!("traefik.http.routers.web--myapp-{idx}-blocked");
            assert_eq!(
                labels[&format!("{r}.rule")],
                format!(
                    "Host(`{domain}`) && (PathPrefix(`/.git/`) || PathPrefix(`/core/install.php`))"
                )
            );
            assert_eq!(labels[&format!("{r}.priority")], "10000");
            assert_eq!(labels[&format!("{r}.service")], "web--myapp");
            // Only the block middleware: no basic auth prompt before the 403.
            assert_eq!(labels[&format!("{r}.middlewares")], "web--myapp--blocked");
            assert_eq!(labels[&format!("{r}.tls")], "true");
            assert_eq!(labels[&format!("{r}.tls.certresolver")], "myresolver");
            // The normal router is unchanged.
            assert_eq!(
                labels[&format!("traefik.http.routers.web--myapp-{idx}.rule")],
                format!("Host(`{domain}`)")
            );
        }
    }

    #[test]
    fn test_traefik_blocked_paths_without_tls() {
        let labels = blocked_override(false, &["/.git/"]);
        assert!(labels.contains_key("traefik.http.routers.web--myapp-0-blocked.rule"));
        assert!(!labels.keys().any(|k| k.ends_with(".tls")));
    }

    #[test]
    fn test_traefik_no_blocked_paths_no_deny_labels() {
        let labels = blocked_override(true, &[]);
        assert!(!labels.keys().any(|k| k.contains("blocked")), "{labels:?}");
    }

    #[test]
    fn test_traefik_deny_routers_are_not_domains() {
        let labels = blocked_override(true, &["/core/install.php"]);
        let insights = ContainerInspectResponse {
            config: Some(bollard_stubs::models::ContainerConfig {
                labels: Some(labels),
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut domains = TraefikLoadBalancer.get_load_balancer_info(insights).domains;
        domains.sort();
        assert_eq!(domains, vec!["a.org", "b.org"]);
    }
}
