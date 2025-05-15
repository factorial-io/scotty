use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::BufReader,
    path::Path,
};

use anyhow;
use serde::{Deserialize, Serialize};
use serde_yml::Value;
use tracing::info;
use utoipa::{ToResponse, ToSchema};

use crate::{
    notification_types::NotificationReceiver,
    settings::{app_blueprint::AppBlueprintMap, apps::Apps},
};

use super::super::create_app_request::CustomDomainMapping;
use super::{service::ServicePortMapping, ttl::AppTtl};

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
            let file = File::open(settings_path).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to open settings file {}: {}",
                    settings_path.display(),
                    e
                )
            })?;
            let reader = BufReader::new(file);
            let yaml: Value = serde_yml::from_reader(reader).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse YAML from {}: {}",
                    settings_path.display(),
                    e
                )
            })?;
            let settings: AppSettings = serde_yml::from_value(yaml).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to deserialize settings from {}: {}",
                    settings_path.display(),
                    e
                )
            })?;

            info!(
                "Successfully read app-settings from {}",
                &settings_path.display()
            );
            Ok(Some(settings))
        } else {
            info!("No settings file found at {}", &settings_path.display());
            Ok(None)
        }
    }
}
