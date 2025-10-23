use chrono::TimeDelta;
use serde::{Deserialize, Serialize};
use tracing::info;
use utoipa::{ToResponse, ToSchema};

use crate::notification_types::NotificationReceiver;
use crate::utils::{format::sanitize_env_var_name, secret::SecretHashMap, slugify::slugify};

use super::container::ContainerState;
use super::settings::AppSettings;
use super::status::{get_app_status_from_services, AppStatus};
use super::ttl::AppTtl;

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
        let settings_yaml = serde_norway::to_string(&self.settings)?;
        tokio::fs::write(&settings_path, settings_yaml).await?;

        Ok(())
    }

    pub fn augment_environment(&self, environment: SecretHashMap) -> SecretHashMap {
        let mut environment = environment;
        environment.insert("SCOTTY__APP_NAME".to_string(), self.name.to_string());

        for service in &self.services {
            let urls = service.get_urls();
            if !urls.is_empty() {
                let name = format!(
                    "SCOTTY__PUBLIC_URL__{}",
                    sanitize_env_var_name(&service.service)
                );
                environment.insert(name, urls[0].to_string());
            }
        }
        environment
    }

    pub fn get_environment(&self) -> SecretHashMap {
        let environment = self
            .settings
            .as_ref()
            .map(|s| s.environment.clone())
            .unwrap_or_default();

        self.augment_environment(environment)
    }

    pub async fn create_settings_from_runtime(
        &self,
        env: &SecretHashMap,
    ) -> anyhow::Result<AppData> {
        let mut new_settings = AppSettings {
            environment: env.clone(),
            ..AppSettings::default()
        };

        let mut basic_auth = None;
        // Iterate over services and add them to the new settings
        for service in &self.services {
            if !service.domains.is_empty() {
                new_settings
                    .public_services
                    .push(super::service::ServicePortMapping {
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
