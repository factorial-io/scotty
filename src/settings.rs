#![allow(dead_code)]

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::{collections::HashMap, env};

#[derive(Debug, Clone)]
#[allow(unused)]
pub enum SchedulerInterval {
    Seconds(u32),
    Minutes(u32),
    Hours(u32),
}
use serde::{de::Error, Deserializer};

#[derive(Debug, Deserialize, Clone)]
pub enum LoadBalancerType {
    HaproxyConfig,
    Traefik,
}

impl<'de> Deserialize<'de> for SchedulerInterval {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        let (num, unit) = s.split_at(s.len() - 1);
        let num: u32 = num.parse().map_err(D::Error::custom)?;

        match unit {
            "s" => Ok(SchedulerInterval::Seconds(num)),
            "m" => Ok(SchedulerInterval::Minutes(num)),
            "h" => Ok(SchedulerInterval::Hours(num)),
            _ => Err(D::Error::custom("Invalid time unit")),
        }
    }
}

impl From<SchedulerInterval> for clokwerk::Interval {
    fn from(val: SchedulerInterval) -> Self {
        match val {
            SchedulerInterval::Seconds(s) => clokwerk::Interval::Seconds(s),
            SchedulerInterval::Minutes(m) => clokwerk::Interval::Minutes(m),
            SchedulerInterval::Hours(h) => clokwerk::Interval::Hours(h),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
#[readonly::make]
pub struct ApiServer {
    pub bind_address: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
#[readonly::make]
pub struct Scheduler {
    pub running_app_check: SchedulerInterval,
    pub ttl_check: SchedulerInterval,
    pub task_cleanup: SchedulerInterval,
}

#[derive(Clone, Deserialize, Debug, Hash, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub enum ActionName {
    PostCreate,
    PostRun,
    PostRebuild,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct AppBlueprint {
    pub name: String,
    pub description: String,
    pub actions: HashMap<ActionName, HashMap<String, Vec<String>>>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
#[readonly::make]
pub struct Apps {
    pub root_folder: String,
    pub max_depth: u32,
    pub domain_suffix: String,
    pub use_tls: bool,
    pub blueprints: HashMap<String, AppBlueprint>,
}

#[derive(Debug, Deserialize, Clone)]
pub enum DockerConnectOptions {
    Socket,
    Local,
    Http,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct TraefikSettings {
    pub use_tls: bool,
    pub network: String,
    pub certresolver: Option<String>,
}

impl TraefikSettings {
    pub fn new(use_tls: bool, network: String, certresolver: Option<String>) -> Self {
        Self {
            use_tls,
            network,
            certresolver,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct HaproxyConfigSettings {
    pub use_tls: bool,
}

impl HaproxyConfigSettings {
    pub fn new(use_tls: bool) -> Self {
        Self { use_tls }
    }
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

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Settings {
    pub debug: bool,
    pub telemetry: Option<String>,
    pub api: ApiServer,
    pub scheduler: Scheduler,
    pub apps: Apps,
    pub docker: DockerSettings,
    pub load_balancer_type: LoadBalancerType,
    pub traefik: TraefikSettings,
    pub haproxy: HaproxyConfigSettings,
}
impl Default for Settings {
    fn default() -> Self {
        Settings {
            debug: false,
            telemetry: None,
            api: ApiServer {
                bind_address: "0.0.0.0:21342".to_string(),
            },
            scheduler: Scheduler {
                running_app_check: SchedulerInterval::Minutes(1),
                ttl_check: SchedulerInterval::Hours(1),
                task_cleanup: SchedulerInterval::Minutes(1),
            },
            apps: Apps {
                root_folder: ".".to_string(),
                max_depth: 3,
                domain_suffix: "".to_string(),
                use_tls: false,
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
        }
    }
}

impl Settings {
    pub fn get_environment() -> Environment {
        Environment::default()
            .prefix("YAFBDS")
            .prefix_separator("__")
            .separator("__")
            .try_parsing(true)
    }

    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("YAFBDS_RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = Config::builder()
            .set_default("api.bind_address", "0.0.0.0:8080")?
            .set_default("apps.max_depth", 3u32)?
            .set_default("docker.connection", "local")?
            // Start off by merging in the "default" configuration file
            .add_source(File::with_name("config/default"))
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
    use super::*;
    use std::env;

    #[test]
    fn test_docker_registry_password_from_env() {
        env::set_var(
            "YAFBDS__DOCKER__REGISTRIES__TEST__PASSWORD",
            "test_password",
        );

        let settings = Config::builder()
            // Add in `./Settings.toml`
            .add_source(config::File::with_name(
                "tests/test_docker_registry_password.yaml",
            ))
            // Add in settings from the environment (with a prefix of APP)
            // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
            .add_source(Settings::get_environment())
            .build()
            .unwrap();

        let settings: Settings = settings.try_deserialize().unwrap();
        assert_eq!(
            settings.docker.registries.get("test").unwrap().password,
            "test_password"
        );

        env::remove_var("YAFBDS__DOCKER__REGISTRIES__TEST__PASSWORD");

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
