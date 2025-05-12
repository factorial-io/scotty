use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    fs::File,
    io::BufReader,
    path::Path,
};

use bollard::secret::ContainerStateStatusEnum;
use chrono::TimeDelta;
use serde::{Deserialize, Serialize};
use serde_yml::Value;
use tracing::info;
use utoipa::{ToResponse, ToSchema};

#[derive(Debug, Serialize, Clone, ToSchema, ToResponse)]
pub struct ServicePortMapping {
    pub service: String,
    pub port: u32,
    pub domains: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum DomainField {
    Single { domain: String },
    Multiple { domains: Vec<String> },
}

impl<'de> Deserialize<'de> for ServicePortMapping {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize the incoming JSON into a temporary map
        #[derive(Deserialize)]
        struct Temp {
            service: String,
            port: u32,
            #[serde(flatten)]
            domain_field: Option<DomainField>,
        }

        // Use the Temp struct to parse and transform into ServicePortMapping
        let Temp {
            service,
            port,
            domain_field,
        } = Temp::deserialize(deserializer)?;

        // Map the domain field to the `domains` field in ServicePortMapping
        let domains = match domain_field {
            None => vec![],
            Some(DomainField::Single { domain }) => vec![domain],
            Some(DomainField::Multiple { domains }) => domains,
        };

        Ok(ServicePortMapping {
            service,
            port,
            domains,
        })
    }
}

#[derive(Debug, PartialEq, Deserialize, Clone, ToSchema, ToResponse)]
pub enum AppTtl {
    Hours(u32),
    Days(u32),
    Forever,
}

impl From<AppTtl> for u32 {
    fn from(val: AppTtl) -> Self {
        match val {
            AppTtl::Hours(h) => h * 3600,
            AppTtl::Days(d) => d * 86400,
            AppTtl::Forever => u32::MAX,
        }
    }
}

impl From<u64> for AppTtl {
    fn from(val: u64) -> Self {
        match val {
            x if x == u64::MAX => AppTtl::Forever,
            x if x % 86400 == 0 => AppTtl::Days((x / 86400) as u32),
            x => AppTtl::Hours((x / 3600) as u32),
        }
    }
}
impl Serialize for AppTtl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            AppTtl::Hours(h) => serializer.serialize_newtype_variant("AppTtl", 0, "Hours", &h),
            AppTtl::Days(d) => serializer.serialize_newtype_variant("AppTtl", 1, "Days", &d),
            AppTtl::Forever => serializer.serialize_unit_variant("AppTtl", 2, "Forever"),
        }
    }
}

#[derive(Debug, Deserialize, Clone, ToSchema, ToResponse)]
pub struct AppSettings {
    pub public_services: Vec<ServicePortMapping>,
    pub domain: String,
    pub time_to_live: AppTtl,
    #[serde(default)]
    pub destroy_on_ttl: bool,
    pub basic_auth: Option<(String, String)>,
    pub disallow_robots: bool,
    pub environment: HashMap<String, String>,
    pub registry: Option<String>,
    pub app_blueprint: Option<String>,
    #[serde(default)]
    pub notify: HashSet<NotificationReceiver>,
}

// Implement Serialize manually with redaction for sensitive environment variables
impl Serialize for AppSettings {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use crate::utils::sensitive_data::mask_sensitive_env_map;
        use serde::ser::SerializeStruct;

        // Create masked environment variables using the utility function
        let masked_env = mask_sensitive_env_map(&self.environment);

        // Serialize with the struct serializer
        let mut state = serializer.serialize_struct("AppSettings", 10)?;
        state.serialize_field("public_services", &self.public_services)?;
        state.serialize_field("domain", &self.domain)?;
        state.serialize_field("time_to_live", &self.time_to_live)?;
        state.serialize_field("destroy_on_ttl", &self.destroy_on_ttl)?;
        state.serialize_field("basic_auth", &self.basic_auth)?;
        state.serialize_field("disallow_robots", &self.disallow_robots)?;
        state.serialize_field("environment", &masked_env)?;
        state.serialize_field("registry", &self.registry)?;
        state.serialize_field("app_blueprint", &self.app_blueprint)?;
        state.serialize_field("notify", &self.notify)?;
        state.end()
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            public_services: Vec::new(),
            domain: "".to_string(),
            time_to_live: AppTtl::Days(7),
            destroy_on_ttl: false,
            basic_auth: None,
            disallow_robots: true,
            environment: HashMap::new(),
            registry: None,
            app_blueprint: None,
            notify: HashSet::new(),
        }
    }
}

impl AppSettings {
    pub fn merge_with_global_settings(&self, setting: &Apps, app_name: &str) -> AppSettings {
        AppSettings {
            domain: format!("{}.{}", app_name, setting.domain_suffix),
            ..self.clone()
        }
    }

    pub fn apply_blueprint(&self, blueprints: &AppBlueprintMap) -> anyhow::Result<AppSettings> {
        if let Some(blueprint_name) = &self.app_blueprint {
            let bp = blueprints
                .get(blueprint_name)
                .ok_or_else(|| anyhow::anyhow!("Blueprint {} not found", blueprint_name))?;
            if let Some(public_services) = &bp.public_services {
                if self.public_services.is_empty() {
                    let mut new_settings = self.clone();
                    new_settings.public_services = public_services
                        .iter()
                        .map(|(service, port)| ServicePortMapping {
                            service: service.clone(),
                            port: *port as u32,
                            domains: vec![],
                        })
                        .collect();
                    return Ok(new_settings);
                }
            }
        }
        Ok(self.clone())
    }

    pub fn apply_custom_domains(
        &self,
        custom_domains: &Vec<CustomDomainMapping>,
    ) -> anyhow::Result<AppSettings> {
        let mut new_settings = self.clone();
        for custom_domain in custom_domains {
            let mut found = false;
            for service in &mut new_settings.public_services {
                if service.service == custom_domain.service {
                    service.domains.push(custom_domain.domain.clone());
                    found = true;
                }
            }
            if !found {
                return Err(anyhow::anyhow!(
                    "Service {} for custom domain {} not found",
                    &custom_domain.service,
                    &custom_domain.domain
                ));
            }
        }
        Ok(new_settings)
    }

    pub fn from_file(settings_path: &Path) -> anyhow::Result<Option<AppSettings>> {
        if settings_path.exists() {
            info!(
                "Trying to read app-settings from {}",
                &settings_path.display()
            );
            let result: anyhow::Result<AppSettings> = {
                let file = File::open(settings_path)?;
                let reader = BufReader::new(file);
                let yaml: Value = serde_yml::from_reader(reader)?;
                let settings: AppSettings = serde_yml::from_value(yaml)?;
                info!(
                    "Successfully read app-settings from {}",
                    &settings_path.display()
                );

                Ok(settings)
            };
            match result {
                Ok(settings) => Ok(Some(settings)),
                Err(e) => {
                    let msg = format!(
                        "Failed to read settings from {}: {}",
                        settings_path.display(),
                        e
                    );
                    Err(anyhow::anyhow!(msg))
                }
            }
        } else {
            info!("No settings file found at {}", &settings_path.display());
            Ok(None)
        }
    }
}

use crate::{
    notification_types::NotificationReceiver,
    settings::{app_blueprint::AppBlueprintMap, apps::Apps},
    utils::slugify::slugify,
};

use super::create_app_request::CustomDomainMapping;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse, PartialEq)]
pub enum ContainerStatus {
    Empty,
    Created,
    Restarting,
    Running,
    Paused,
    Removing,
    Exited,
    Dead,
}

impl Display for ContainerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerStatus::Empty => write!(f, "empty"),
            ContainerStatus::Created => write!(f, "created"),
            ContainerStatus::Restarting => write!(f, "restarting"),
            ContainerStatus::Running => write!(f, "running"),
            ContainerStatus::Paused => write!(f, "paused"),
            ContainerStatus::Removing => write!(f, "removing"),
            ContainerStatus::Exited => write!(f, "exited"),
            ContainerStatus::Dead => write!(f, "dead"),
        }
    }
}

impl From<ContainerStateStatusEnum> for ContainerStatus {
    fn from(status: ContainerStateStatusEnum) -> Self {
        match status {
            ContainerStateStatusEnum::EMPTY => ContainerStatus::Empty,
            ContainerStateStatusEnum::CREATED => ContainerStatus::Created,
            ContainerStateStatusEnum::RESTARTING => ContainerStatus::Restarting,
            ContainerStateStatusEnum::RUNNING => ContainerStatus::Running,
            ContainerStateStatusEnum::PAUSED => ContainerStatus::Paused,
            ContainerStateStatusEnum::REMOVING => ContainerStatus::Removing,
            ContainerStateStatusEnum::EXITED => ContainerStatus::Exited,
            ContainerStateStatusEnum::DEAD => ContainerStatus::Dead,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse)]
pub struct ContainerState {
    pub status: ContainerStatus,
    pub id: Option<String>,
    pub service: String,
    pub domains: Vec<String>,
    pub use_tls: bool,
    pub port: Option<u32>,
    pub started_at: Option<chrono::DateTime<chrono::Local>>,
    pub used_registry: Option<String>,
    pub basic_auth: Option<(String, String)>,
}

impl Default for ContainerState {
    fn default() -> Self {
        ContainerState {
            status: ContainerStatus::Empty,
            id: None,
            service: "".to_string(),
            domains: vec![],
            use_tls: false,
            port: None,
            started_at: None,
            used_registry: None,
            basic_auth: None,
        }
    }
}

impl ContainerState {
    pub fn is_running(&self) -> bool {
        self.status == ContainerStatus::Running
            || self.status == ContainerStatus::Created
            || self.status == ContainerStatus::Restarting
    }

    pub fn running_since(&self) -> Option<TimeDelta> {
        self.started_at
            .map(|started_at| chrono::Local::now() - started_at)
    }

    pub fn get_urls(&self) -> Vec<String> {
        self.domains
            .iter()
            .map(|domain| {
                if self.use_tls {
                    format!("https://{}", domain)
                } else {
                    format!("http://{}", domain)
                }
            })
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse, PartialEq, Eq)]
pub enum AppStatus {
    Stopped,
    Starting,
    Running,
    Creating,
    Destroying,
    Unsupported,
}

impl std::fmt::Display for AppStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AppStatus::Stopped => write!(f, "Stopped"),
            AppStatus::Starting => write!(f, "Starting"),
            AppStatus::Running => write!(f, "Running"),
            AppStatus::Creating => write!(f, "Creating"),
            AppStatus::Destroying => write!(f, "Destroying"),
            AppStatus::Unsupported => write!(f, "Unsupported"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse)]
pub struct AppData {
    pub status: AppStatus,
    pub name: String,
    pub root_directory: String,
    pub docker_compose_path: String,
    pub services: Vec<ContainerState>,
    pub settings: Option<AppSettings>,
    pub last_checked: Option<chrono::DateTime<chrono::Local>>,
}

impl Default for AppData {
    /// Creates a default `AppData` instance with empty fields, stopped status, and no last checked timestamp.
    ///
    /// # Examples
    ///
    /// ```
    /// use scotty_core::apps::app_data::AppData;
    /// use scotty_core::apps::app_data::AppStatus;
    ///
    /// let app = AppData::default();
    /// assert_eq!(app.status, AppStatus::Stopped);
    /// assert!(app.name.is_empty());
    /// assert!(app.last_checked.is_none());
    /// ```
    fn default() -> Self {
        AppData {
            status: AppStatus::Stopped,
            name: "".to_string(),
            root_directory: "".to_string(),
            docker_compose_path: "".to_string(),
            services: Vec::new(),
            settings: None,
            last_checked: None,
        }
    }
}

impl AppData {
    /// Constructs a new `AppData` instance with the provided name, directories, services, and settings.
    ///
    /// The application name is slugified, and the overall status is determined from the service states. The `last_checked` field is initialized to `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use scotty_core::apps::app_data::AppData;
    /// let services = vec![];
    /// let app_data = AppData::new("My App", "/apps/my_app", "/apps/my_app/docker-compose.yml", services, None);
    /// assert_eq!(app_data.name, "my-app");
    /// assert!(app_data.last_checked.is_none());
    /// ```
    pub fn new(
        name: &str,
        root_directory: &str,
        docker_compose_path: &str,
        services: Vec<ContainerState>,
        settings: Option<AppSettings>,
    ) -> AppData {
        AppData {
            status: get_app_status_from_services(&services),
            name: slugify(name),
            root_directory: root_directory.to_string(),
            docker_compose_path: docker_compose_path.to_string(),
            services,
            settings,
            last_checked: None,
        }
    }

    pub fn urls(&self) -> Vec<String> {
        self.services.iter().flat_map(|s| s.get_urls()).collect()
    }

    pub fn get_ttl(&self) -> AppTtl {
        self.settings
            .as_ref()
            .map(|s| s.time_to_live.clone())
            .unwrap_or(AppTtl::Days(7))
    }
    pub fn running_since(&self) -> Option<TimeDelta> {
        let mut since: Option<TimeDelta> = None;
        for service in &self.services {
            if let Some(delta) = service.running_since() {
                since = Some(delta);
            }
        }

        since
    }

    pub fn get_registry(&self) -> Option<String> {
        if let Some(settings) = &self.settings {
            return settings.registry.clone();
        }
        self.services.iter().find_map(|s| s.used_registry.clone())
    }

    pub fn add_notifications(&self, service_ids: &[NotificationReceiver]) -> AppData {
        let mut new_settings = self.settings.clone().unwrap_or_default();
        for id in service_ids {
            new_settings.notify.insert(id.clone());
        }
        AppData {
            settings: Some(new_settings),
            ..self.clone()
        }
    }

    pub fn remove_notifications(&self, service_ids: &[NotificationReceiver]) -> AppData {
        let mut new_settings = self.settings.clone().unwrap_or_default();
        new_settings.notify.retain(|x| !service_ids.contains(x));
        AppData {
            settings: Some(new_settings),
            ..self.clone()
        }
    }

    pub async fn save_settings(&self) -> anyhow::Result<()> {
        let root_directory = std::path::PathBuf::from(&self.root_directory);

        let settings_path = root_directory.join(".scotty.yml");
        info!("Saving settings to {}", settings_path.display());
        let settings_yaml = serde_yml::to_string(&self.settings)?;
        tokio::fs::write(&settings_path, settings_yaml).await?;

        Ok(())
    }

    pub fn augment_environment(
        &self,
        environment: HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut environment = environment;
        environment.insert("SCOTTY__APP_NAME".to_string(), self.name.to_string());

        for service in &self.services {
            let urls = service.get_urls();
            if !urls.is_empty() {
                let name = format!("SCOTTY__PUBLIC_URL__{}", service.service.to_uppercase());
                environment.insert(name, urls[0].to_string());
            }
        }
        environment
    }

    pub fn get_environment(&self) -> HashMap<String, String> {
        let environment = self
            .settings
            .as_ref()
            .map(|s| s.environment.clone())
            .unwrap_or_default();

        self.augment_environment(environment)
    }

    pub async fn create_settings_from_runtime(
        &self,
        env: &HashMap<String, String>,
    ) -> anyhow::Result<AppData> {
        let mut new_settings = AppSettings {
            environment: env.clone(),
            ..AppSettings::default()
        };

        let mut basic_auth = None;
        // Iterate over services and add them to the new settings
        for service in &self.services {
            if !service.domains.is_empty() {
                new_settings.public_services.push(ServicePortMapping {
                    service: service.service.clone(),
                    port: service.port.unwrap(),
                    domains: service.domains.clone(),
                });
            }
            if service.basic_auth.is_some() {
                basic_auth = service.basic_auth.clone();
            }
        }

        new_settings.basic_auth = basic_auth;

        let app_data = AppData {
            settings: Some(new_settings),
            ..self.clone()
        };
        app_data.save_settings().await?;
        Ok(app_data)
    }
}

fn count_state(services: &[ContainerState], required: ContainerStatus) -> usize {
    services
        .iter()
        .fold(0, |acc, x| if x.status == required { acc + 1 } else { acc })
}

fn get_app_status_from_services(services: &[ContainerState]) -> AppStatus {
    let count_running_services = count_state(services, ContainerStatus::Running);
    match count_running_services {
        0 => AppStatus::Stopped,
        x if x == services.len() => AppStatus::Running,
        _ => AppStatus::Starting,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_service_port_mapping_deserialization() {
        // Test no domain
        let json = json!({
            "service": "web",
            "port": 8080,
        });
        let mapping: ServicePortMapping = serde_json::from_value(json).unwrap();
        assert_eq!(mapping.service, "web");
        assert_eq!(mapping.port, 8080);
        assert_eq!(mapping.domains.len(), 0);

        // Test single domain
        let json = json!({
            "service": "web",
            "port": 8080,
            "domain": "example.com"
        });
        let mapping: ServicePortMapping = serde_json::from_value(json).unwrap();
        assert_eq!(mapping.service, "web");
        assert_eq!(mapping.port, 8080);
        assert_eq!(mapping.domains, vec!["example.com"]);

        // Test multiple domains
        let json = json!({
            "service": "api",
            "port": 3000,
            "domains": ["api1.com", "api2.com"]
        });
        let mapping: ServicePortMapping = serde_json::from_value(json).unwrap();
        assert_eq!(mapping.service, "api");
        assert_eq!(mapping.port, 3000);
        assert_eq!(mapping.domains, vec!["api1.com", "api2.com"]);
    }
}
