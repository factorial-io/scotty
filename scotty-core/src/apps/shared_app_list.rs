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
}
