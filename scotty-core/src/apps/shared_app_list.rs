#![allow(dead_code)]

use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use tracing::instrument;

use super::app_data::{AppData, LoadBalancerConnectivity};

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

    /// Patches only the load-balancer connectivity of one app.
    ///
    /// Deliberately *not* an `update_app` with a modified copy: the proxy-network
    /// reconciler works from a snapshot taken before a round of Docker calls, and
    /// writing that whole snapshot back would revert any field another writer
    /// (a state machine transition, a notification change) touched in the
    /// meantime. Read-modify-write happens here under a single write lock, so the
    /// only field this can ever change is the one it is named after.
    ///
    /// Returns `true` when the app exists and the value actually changed, so
    /// callers can skip broadcasting a no-op update.
    pub async fn set_load_balancer_connectivity(
        &self,
        app_name: &str,
        connectivity: LoadBalancerConnectivity,
    ) -> bool {
        let mut apps = self.apps.write().await;
        match apps.get_mut(app_name) {
            Some(app) if app.load_balancer_connectivity != connectivity => {
                app.load_balancer_connectivity = connectivity;
                true
            }
            _ => false,
        }
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
                exit_code: None,
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

    /// The reconciler snapshots the app list, does Docker I/O, and only then
    /// writes connectivity back. Other writers (state machine transitions,
    /// notification changes) touch the same cache in that window, so the patch
    /// must not carry a stale copy of the rest of the app with it.
    #[tokio::test]
    async fn set_load_balancer_connectivity_does_not_revert_concurrent_changes() {
        use crate::apps::app_data::AppStatus;

        let list = SharedAppList::new();
        let mut app = make_app_with_settings("myapp", "myapp.example.com", vec![]);
        app.status = AppStatus::Creating;
        list.add_app(app.clone()).await.unwrap();

        // A reconciliation pass takes its snapshot here (`app`, still `Creating`).

        // Meanwhile a state machine finishes the deploy and stores the new status.
        let mut deployed = app.clone();
        deployed.status = AppStatus::Running;
        list.update_app(deployed).await.unwrap();

        // The pass now writes back what it observed.
        assert!(
            list.set_load_balancer_connectivity("myapp", LoadBalancerConnectivity::Connected)
                .await
        );

        let stored = list.get_app("myapp").await.unwrap();
        assert_eq!(
            stored.load_balancer_connectivity,
            LoadBalancerConnectivity::Connected
        );
        // The concurrent update survives instead of being reverted to `Creating`.
        assert_eq!(stored.status, AppStatus::Running);
    }

    #[tokio::test]
    async fn set_load_balancer_connectivity_reports_whether_it_changed_anything() {
        let list = SharedAppList::new();
        let app = make_app_with_settings("myapp", "myapp.example.com", vec![]);
        list.add_app(app).await.unwrap();

        // First write changes it, the second is a no-op the caller can skip
        // broadcasting.
        assert!(
            list.set_load_balancer_connectivity("myapp", LoadBalancerConnectivity::Connected)
                .await
        );
        assert!(
            !list
                .set_load_balancer_connectivity("myapp", LoadBalancerConnectivity::Connected)
                .await
        );
        // An app that is not in the cache (destroyed mid-pass) is not resurrected.
        assert!(
            !list
                .set_load_balancer_connectivity("gone", LoadBalancerConnectivity::Connected)
                .await
        );
        assert!(list.get_app("gone").await.is_none());
    }
}
