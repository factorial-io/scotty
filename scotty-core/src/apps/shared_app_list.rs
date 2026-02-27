#![allow(dead_code)]

use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use tracing::instrument;

use super::app_data::AppData;

pub type AppHashMap = HashMap<String, AppData>;

#[derive(
    Debug,
    serde::Serialize,
    serde::Deserialize,
    utoipa::IntoParams,
    utoipa::ToSchema,
    utoipa::ToResponse,
)]
pub struct AppDataVec {
    pub apps: Vec<AppData>,
}

#[derive(Debug, Clone)]
pub struct SharedAppList {
    apps: Arc<RwLock<AppHashMap>>,
}

impl Default for SharedAppList {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedAppList {
    pub fn new() -> SharedAppList {
        SharedAppList {
            apps: Arc::new(RwLock::new(AppHashMap::new())),
        }
    }

    pub async fn add_app(&self, app: AppData) -> anyhow::Result<()> {
        self.apps.write().await.insert(app.name.clone(), app);
        Ok(())
    }

    pub async fn remove_app(&self, app_name: &str) -> anyhow::Result<()> {
        self.apps.write().await.remove(app_name);
        Ok(())
    }

    pub async fn has_app(&self, app_name: &str) -> bool {
        self.apps.read().await.contains_key(app_name)
    }

    pub async fn get_app(&self, app_name: &str) -> Option<AppData> {
        let t = self.apps.read().await;
        t.get(app_name).cloned()
    }

    #[instrument]
    pub async fn get_apps(&self) -> AppDataVec {
        let t = self.apps.read().await;
        AppDataVec {
            apps: t.values().cloned().collect(),
        }
    }

    #[instrument]
    pub async fn set_apps(&self, new_apps: &AppDataVec) -> anyhow::Result<()> {
        let mut t = self.apps.write().await;
        t.clear();
        t.extend(
            new_apps
                .apps
                .iter()
                .map(|app| (app.name.clone(), app.clone())),
        );

        Ok(())
    }

    #[instrument]
    pub async fn update_app(&self, app: AppData) -> anyhow::Result<AppData> {
        self.apps
            .write()
            .await
            .insert(app.name.clone(), app.clone());
        Ok(app)
    }

    pub async fn len(&self) -> usize {
        let t = self.apps.read().await;
        t.len()
    }

    pub async fn is_empty(&self) -> bool {
        let t = self.apps.read().await;
        t.is_empty()
    }

    /// Look up an app by one of its domains.
    ///
    /// Searches through all apps' settings (configured and auto-generated domains)
    /// and container states (runtime domains from Traefik labels).
    /// Domain comparison is case-insensitive per RFC 4343.
    ///
    /// Note: holds the read lock for the entire scan including `.clone()` on match.
    /// This is acceptable for a micro-PaaS with a small number of apps.
    pub async fn find_app_by_domain(&self, domain: &str) -> Option<AppData> {
        let apps = self.apps.read().await;
        for app in apps.values() {
            // Check settings-based domains (custom and auto-generated)
            if let Some(settings) = &app.settings {
                for service in &settings.public_services {
                    if service
                        .get_domains(&settings.domain)
                        .iter()
                        .any(|d| d.eq_ignore_ascii_case(domain))
                    {
                        return Some(app.clone());
                    }
                }
            }

            // Check container-level domains (from running/previously-running state)
            for container in &app.services {
                if container
                    .domains
                    .iter()
                    .any(|d| d.eq_ignore_ascii_case(domain))
                {
                    return Some(app.clone());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::app_data::{AppSettings, ContainerState, ContainerStatus, ServicePortMapping};

    fn make_app_with_settings(
        name: &str,
        domain: &str,
        services: Vec<ServicePortMapping>,
    ) -> AppData {
        AppData {
            name: name.to_string(),
            settings: Some(AppSettings {
                domain: domain.to_string(),
                public_services: services,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn make_app_with_containers(name: &str, containers: Vec<ContainerState>) -> AppData {
        AppData {
            name: name.to_string(),
            services: containers,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_find_app_by_custom_domain() {
        let list = SharedAppList::new();
        let app = make_app_with_settings(
            "myapp",
            "myapp.example.com",
            vec![ServicePortMapping {
                service: "web".to_string(),
                port: 8080,
                domains: vec!["custom.example.com".to_string()],
            }],
        );
        list.add_app(app).await.unwrap();

        let found = list.find_app_by_domain("custom.example.com").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "myapp");
    }

    #[tokio::test]
    async fn test_find_app_by_auto_generated_domain() {
        let list = SharedAppList::new();
        let app = make_app_with_settings(
            "myapp",
            "myapp.example.com",
            vec![ServicePortMapping {
                service: "web".to_string(),
                port: 8080,
                domains: vec![],
            }],
        );
        list.add_app(app).await.unwrap();

        // Auto-generated domain is {service}.{settings.domain}
        let found = list.find_app_by_domain("web.myapp.example.com").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "myapp");
    }

    #[tokio::test]
    async fn test_find_app_by_container_domain() {
        let list = SharedAppList::new();
        let app = make_app_with_containers(
            "myapp",
            vec![ContainerState {
                status: ContainerStatus::Running,
                id: None,
                service: "web".to_string(),
                domains: vec!["runtime.example.com".to_string()],
                use_tls: false,
                port: Some(8080),
                started_at: None,
                used_registry: None,
                basic_auth: None,
            }],
        );
        list.add_app(app).await.unwrap();

        let found = list.find_app_by_domain("runtime.example.com").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "myapp");
    }

    #[tokio::test]
    async fn test_find_app_by_domain_not_found() {
        let list = SharedAppList::new();
        let app = make_app_with_settings(
            "myapp",
            "myapp.example.com",
            vec![ServicePortMapping {
                service: "web".to_string(),
                port: 8080,
                domains: vec![],
            }],
        );
        list.add_app(app).await.unwrap();

        let found = list.find_app_by_domain("unknown.example.com").await;
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_find_app_by_domain_empty_list() {
        let list = SharedAppList::new();
        let found = list.find_app_by_domain("any.example.com").await;
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_find_app_by_domain_case_insensitive() {
        let list = SharedAppList::new();
        let app_custom = make_app_with_settings(
            "myapp",
            "myapp.example.com",
            vec![ServicePortMapping {
                service: "web".to_string(),
                port: 8080,
                domains: vec!["Custom.Example.COM".to_string()],
            }],
        );
        let app_auto = make_app_with_settings(
            "otherapp",
            "otherapp.example.com",
            vec![ServicePortMapping {
                service: "api".to_string(),
                port: 3000,
                domains: vec![],
            }],
        );
        list.add_app(app_custom).await.unwrap();
        list.add_app(app_auto).await.unwrap();

        // Lowercase lookup should match uppercase stored custom domain
        let found = list.find_app_by_domain("custom.example.com").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "myapp");

        // Uppercase lookup should match auto-generated domain
        let found = list.find_app_by_domain("API.OTHERAPP.EXAMPLE.COM").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "otherapp");
    }
}
