use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::BufReader,
    path::Path,
};

use anyhow;
use serde::{Deserialize, Serialize};
use serde_norway::Value;
use tracing::info;
use utoipa::{ToResponse, ToSchema};

use crate::{
    notification_types::NotificationReceiver,
    settings::{app_blueprint::AppBlueprintMap, apps::Apps},
    utils::domain_hash,
};

use super::super::create_app_request::CustomDomainMapping;
use super::{service::ServicePortMapping, ttl::AppTtl};

fn default_scopes() -> Vec<String> {
    vec!["default".to_string()]
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema, ToResponse)]
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
    #[serde(default)]
    pub middlewares: Vec<String>,
    #[serde(default = "default_scopes", alias = "groups")]
    pub scopes: Vec<String>,
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
            middlewares: Vec::new(),
            scopes: default_scopes(),
        }
    }
}

impl AppSettings {
    pub fn merge_with_global_settings(&self, setting: &Apps, app_name: &str) -> AppSettings {
        let safe_name = domain_hash::domain_safe_name(app_name);
        AppSettings {
            domain: format!("{}.{}", safe_name, setting.domain_suffix),
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
            let yaml: Value = serde_norway::from_reader(reader).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse YAML from {}: {}",
                    settings_path.display(),
                    e
                )
            })?;
            let settings: AppSettings = serde_norway::from_value(yaml).map_err(|e| {
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

    #[cfg(test)]
    pub fn to_file(&self, settings_path: &Path) -> anyhow::Result<()> {
        let yaml = serde_norway::to_string(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize settings to YAML: {}", e))?;

        std::fs::write(settings_path, yaml).map_err(|e| {
            anyhow::anyhow!(
                "Failed to write settings to {}: {}",
                settings_path.display(),
                e
            )
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::sensitive_data::{is_sensitive, mask_sensitive_env_map};
    use tempfile::tempdir;

    #[test]
    fn test_environment_vars_not_masked_in_yaml_file() {
        // Create AppSettings with sensitive environment variables
        let mut env_vars = HashMap::new();
        env_vars.insert("API_KEY".to_string(), "secret-api-key-12345".to_string());
        env_vars.insert(
            "DATABASE_URL".to_string(),
            "postgres://user:password@localhost/db".to_string(),
        );
        env_vars.insert("NORMAL_VAR".to_string(), "not-sensitive".to_string());

        let settings = AppSettings {
            public_services: vec![],
            domain: "test.example.com".to_string(),
            time_to_live: AppTtl::Days(7),
            disallow_robots: true,
            environment: env_vars,
            ..Default::default()
        };

        // Create a temporary directory for the test
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let settings_path = temp_dir.path().join(".scotty.yml");

        // Save settings to file
        settings
            .to_file(&settings_path)
            .expect("Failed to save settings");

        // Read the settings back from the file
        let loaded_settings = AppSettings::from_file(&settings_path)
            .expect("Failed to load settings")
            .expect("Settings should exist");

        // Verify sensitive environment variables are not masked
        assert_eq!(
            loaded_settings.environment.get("API_KEY").unwrap(),
            "secret-api-key-12345"
        );
        assert_eq!(
            loaded_settings.environment.get("DATABASE_URL").unwrap(),
            "postgres://user:password@localhost/db"
        );
        assert_eq!(
            loaded_settings.environment.get("NORMAL_VAR").unwrap(),
            "not-sensitive"
        );

        // Verify that if we were to mask them, they would be different
        let masked_env = mask_sensitive_env_map(&settings.environment);
        assert_ne!(masked_env.get("API_KEY").unwrap(), "secret-api-key-12345");
        assert_ne!(
            masked_env.get("DATABASE_URL").unwrap(),
            "postgres://user:password@localhost/db"
        );

        // Verify that the sensitive detection is working correctly
        assert!(is_sensitive("API_KEY"));
        assert!(!is_sensitive("NORMAL_VAR"));
    }
}
