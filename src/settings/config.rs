#![allow(dead_code)]

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::{collections::HashMap, env};

use super::{
    api_server::ApiServer,
    app_blueprint::AppBlueprintMap,
    docker::{DockerConnectOptions, DockerSettings},
    loadbalancer::{HaproxyConfigSettings, LoadBalancerType, TraefikSettings},
    notification_services::NotificationService,
    scheduler_interval::SchedulerInterval,
};

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
#[readonly::make]
pub struct Scheduler {
    pub running_app_check: SchedulerInterval,
    pub ttl_check: SchedulerInterval,
    pub task_cleanup: SchedulerInterval,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
#[readonly::make]
pub struct Apps {
    pub root_folder: String,
    pub max_depth: u32,
    pub domain_suffix: String,
    pub blueprints: AppBlueprintMap,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct OnePasswordSettings {
    pub jwt_token: String,
    pub server: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Settings {
    pub debug: bool,
    pub telemetry: Option<String>,
    pub api: ApiServer,
    pub frontend_directory: Option<String>,
    pub scheduler: Scheduler,
    pub apps: Apps,
    pub docker: DockerSettings,
    pub load_balancer_type: LoadBalancerType,
    pub traefik: TraefikSettings,
    pub haproxy: HaproxyConfigSettings,
    #[serde(default)]
    pub onepassword: HashMap<String, OnePasswordSettings>,
    #[serde(default)]
    pub notification_services: HashMap<String, NotificationService>,
}
impl Default for Settings {
    fn default() -> Self {
        Settings {
            debug: false,
            telemetry: None,
            frontend_directory: None,
            api: ApiServer::default(),
            scheduler: Scheduler {
                running_app_check: SchedulerInterval::Minutes(1),
                ttl_check: SchedulerInterval::Hours(1),
                task_cleanup: SchedulerInterval::Minutes(1),
            },
            apps: Apps {
                root_folder: ".".to_string(),
                max_depth: 3,
                domain_suffix: "".to_string(),
                blueprints: HashMap::new(),
            },
            docker: DockerSettings {
                connection: DockerConnectOptions::Local,
                registries: HashMap::new(),
            },
            load_balancer_type: LoadBalancerType::Traefik,
            traefik: TraefikSettings {
                use_tls: false,
                network: "proxy".to_string(),
                certresolver: None,
            },
            haproxy: HaproxyConfigSettings { use_tls: false },
            onepassword: HashMap::new(),
            notification_services: HashMap::new(),
        }
    }
}

impl Settings {
    pub fn get_environment() -> Environment {
        Environment::default()
            .prefix("SCOTTY")
            .prefix_separator("__")
            .separator("__")
            .try_parsing(true)
    }

    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("SCOTTY_RUN_MODE").unwrap_or_else(|_| "development".into());

        let mut builder = Config::builder()
            .set_default("api.bind_address", "0.0.0.0:8080")?
            .set_default("api.create_app_max_size", 10 * 1024 * 1024)?
            .set_default("apps.max_depth", 3u32)?
            .set_default("docker.connection", "local")?
            // Start off by merging in the "default" configuration file
            .add_source(File::with_name("config/default"));

        // Add every file in config/blueprints to the configuration.
        if let Ok(entries) = std::fs::read_dir("config/blueprints") {
            for entry in entries.into_iter().flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "yaml" || ext == "yml" {
                        builder = builder.add_source(File::from(path));
                    }
                }
            }
        }

        // Add the rest of the configuration files.
        let s = builder
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            .add_source(File::with_name("config/local").required(false))
            .add_source(Self::get_environment())
            .build()?;

        let mut settings: Settings = s.try_deserialize()?;

        // Check if we should serve the frontend.
        // We check for special strings here, so we can override and disable
        // config via environment variables, even if set in the default config
        settings.telemetry = settings.check_if_optional(&settings.telemetry);
        settings.apps.root_folder = std::fs::canonicalize(&settings.apps.root_folder)
            .map_err(|e| ConfigError::Message(format!("Failed to get realpath: {}", e)))?
            .to_str()
            .ok_or_else(|| ConfigError::Message("Failed to convert realpath to string".into()))?
            .to_string();
        Ok(settings)
    }

    fn check_if_optional(&self, s: &Option<String>) -> Option<String> {
        match s {
            None => None,
            Some(s) => match s.to_lowercase().as_str() {
                "no" | "false" | "0" => None,
                _ => Some(s.to_string()),
            },
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::settings::app_blueprint::ActionName;

    use super::*;
    use std::env;

    #[test]
    fn test_docker_registry_password_from_env() {
        env::set_var(
            "SCOTTY__DOCKER__REGISTRIES__TEST__PASSWORD",
            "test_password",
        );

        env::set_var("SCOTTY__ONEPASSWARD__TEST__JWT_TOKEN", "test_jwt");

        let settings = Config::builder()
            // Add in `./Settings.toml`
            .add_source(config::File::with_name(
                "tests/test_docker_registry_password.yaml",
            ))
            .add_source(Settings::get_environment())
            .build()
            .unwrap();

        let settings: Settings = settings.try_deserialize().unwrap();
        assert_eq!(
            &settings.docker.registries.get("test").unwrap().password,
            "test_password"
        );
        assert_eq!(
            &settings.onepassword.get("test").unwrap().jwt_token,
            "test_jwt"
        );

        env::remove_var("SCOTTY__DOCKER__REGISTRIES__TEST__PASSWORD");
        env::remove_var("SCOTTY__ONEPASSWORD__TEST__JWT_TOKEN");

        let blueprint = settings.apps.blueprints.get("nginx-lagoon").unwrap();
        assert_eq!(blueprint.name, "NGINX using lagoon base images");
        let script = blueprint
            .actions
            .get(&ActionName::PostCreate)
            .unwrap()
            .get("nginx")
            .unwrap();
        assert_eq!(script[0], "echo \"Hello, World!\"".to_string());
    }
}
