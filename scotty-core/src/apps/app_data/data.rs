use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};
use tracing::info;
use utoipa::{ToResponse, ToSchema};

use crate::notification_types::NotificationReceiver;
use crate::utils::{format::sanitize_env_var_name, secret::SecretHashMap, slugify::slugify};

use super::connectivity::LoadBalancerConnectivity;
use super::container::ContainerState;
use super::settings::AppSettings;
use super::status::{get_app_status_from_services, AppStatus};
use super::ttl::AppTtl;

pub use scotty_types::AppOperation;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, ToResponse)]
pub struct AppData {
    pub status: AppStatus,
    pub name: String,
    pub root_directory: String,
    pub docker_compose_path: String,
    pub services: Vec<ContainerState>,
    pub settings: Option<AppSettings>,
    #[serde(default, with = "scotty_types::time::local_opt")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub last_checked: Option<Timestamp>,
    /// When the server's next scheduled app-state sweep is due, or `None` before the
    /// first sweep has run. Server-global rather than per-app: one scheduler job
    /// inspects every app. It is read from the scheduler instead of extrapolated from
    /// `last_checked`, because `last_checked` also moves when a manual run/stop/rebuild
    /// inspects a single app between sweeps. `serde(default)` keeps the wire format
    /// compatible with clients that predate the field.
    #[serde(default, with = "scotty_types::time::local_opt")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub next_check: Option<Timestamp>,
    /// Whether the load balancer can reach this app. Only the proxy-network
    /// reconciler sets a definite value; every other construction path leaves
    /// it `Unknown`, which is honest rather than optimistic. `serde(default)`
    /// keeps the wire format compatible in both directions with clients that
    /// predate the field.
    #[serde(default)]
    pub load_balancer_connectivity: LoadBalancerConnectivity,
    /// The operation running on the app, if any. Filled from the server's
    /// lock registry when the app is returned, never stored in the app list.
    #[serde(default)]
    pub operation: Option<AppOperation>,
    /// When the TTL checker stops or destroys the app, from `ttl_expiry`.
    /// Filled like `operation` when the app is returned, never stored in the
    /// app list. An absolute time, so cached copies and clients do not go
    /// stale.
    #[serde(default, with = "scotty_types::time::local_opt")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub ttl_expires_at: Option<Timestamp>,
    /// Effective blocked paths (global plus blueprint), from
    /// `effective_blocked_paths`. Filled like `operation` when the app is
    /// returned, never stored in the app list.
    #[serde(default)]
    pub blocked_paths: Vec<String>,
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
            next_check: None,
            load_balancer_connectivity: LoadBalancerConnectivity::default(),
            operation: None,
            ttl_expires_at: None,
            blocked_paths: Vec::new(),
        }
    }
}

impl AppData {
    /// Blocked paths Scotty enforces for this app with the current config.
    /// An app without Scotty settings gets no load balancer config from
    /// Scotty, so nothing is blocked for it.
    pub fn effective_blocked_paths(&self, apps: &crate::settings::apps::Apps) -> Vec<String> {
        self.settings
            .as_ref()
            .map(|s| apps.blocked_paths_for(s.app_blueprint.as_deref()))
            .unwrap_or_default()
    }

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
            next_check: None,
            load_balancer_connectivity: LoadBalancerConnectivity::default(),
            operation: None,
            ttl_expires_at: None,
            blocked_paths: Vec::new(),
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

    /// When the app's TTL runs out: the earliest start of a running service
    /// plus the TTL, as any service running longer than the TTL ends the app.
    /// `None` for `Forever` or without a running service.
    pub fn ttl_expiry(&self) -> Option<Timestamp> {
        let ttl = self.get_ttl();
        if ttl == AppTtl::Forever {
            return None;
        }
        let ttl = SignedDuration::from_secs(u32::from(ttl).into());
        self.services
            .iter()
            .filter(|s| s.is_running())
            .filter_map(|s| s.started_at)
            .min()
            .and_then(|started| started.checked_add(ttl).ok())
    }

    /// Whether the TTL checker destroys the app instead of stopping it.
    pub fn destroys_on_ttl(&self) -> bool {
        self.status != AppStatus::Unsupported
            && self.settings.as_ref().is_some_and(|s| s.destroy_on_ttl)
    }

    pub fn running_since(&self) -> Option<SignedDuration> {
        let mut since: Option<SignedDuration> = None;
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

    /// Find a container state by service name
    pub fn find_container_by_service(&self, service_name: &str) -> Option<&ContainerState> {
        self.services.iter().find(|s| s.service == service_name)
    }

    /// Get the container ID for a service
    pub fn get_container_id_for_service(&self, service_name: &str) -> Option<&String> {
        self.find_container_by_service(service_name)
            .and_then(|container| container.id.as_ref())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::app_data::ContainerStatus;

    fn service(status: ContainerStatus, hours_ago: i64) -> ContainerState {
        ContainerState {
            status,
            started_at: Some(Timestamp::now() - SignedDuration::from_hours(hours_ago)),
            ..Default::default()
        }
    }

    fn app(ttl: AppTtl, services: Vec<ContainerState>) -> AppData {
        AppData {
            services,
            settings: Some(AppSettings {
                time_to_live: ttl,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn expiry_counts_from_the_earliest_running_service() {
        let a = app(
            AppTtl::Hours(10),
            vec![
                service(ContainerStatus::Running, 1),
                service(ContainerStatus::Running, 3),
                service(ContainerStatus::Exited, 9),
            ],
        );
        let started = a.services[1].started_at.unwrap();
        assert_eq!(
            a.ttl_expiry(),
            Some(started + SignedDuration::from_hours(10))
        );
    }

    #[test]
    fn no_expiry_when_forever_or_not_running() {
        let running = vec![service(ContainerStatus::Running, 1)];
        assert_eq!(app(AppTtl::Forever, running).ttl_expiry(), None);
        let stopped = vec![service(ContainerStatus::Exited, 1)];
        assert_eq!(app(AppTtl::Days(7), stopped).ttl_expiry(), None);
        assert_eq!(app(AppTtl::Days(7), vec![]).ttl_expiry(), None);
    }

    #[test]
    fn huge_ttl_does_not_overflow() {
        let a = app(
            AppTtl::Days(u32::MAX),
            vec![service(ContainerStatus::Running, 1)],
        );
        assert!(a.ttl_expiry().is_some_and(|e| e > Timestamp::now()));
    }

    #[test]
    fn app_without_settings_uses_seven_days_and_is_stopped() {
        let a = AppData {
            services: vec![service(ContainerStatus::Running, 0)],
            ..Default::default()
        };
        let started = a.services[0].started_at.unwrap();
        assert_eq!(
            a.ttl_expiry(),
            Some(started + SignedDuration::from_hours(168))
        );
        assert!(!a.destroys_on_ttl());
    }

    #[test]
    fn destroys_only_supported_apps_with_the_setting() {
        let mut a = app(AppTtl::Days(1), vec![]);
        assert!(!a.destroys_on_ttl());
        a.settings.as_mut().unwrap().destroy_on_ttl = true;
        assert!(a.destroys_on_ttl());
        a.status = AppStatus::Unsupported;
        assert!(!a.destroys_on_ttl());
    }
}
