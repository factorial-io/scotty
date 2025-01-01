use std::sync::Arc;

use scotty_core::{
    apps::app_data::AppData,
    tasks::{running_app_context::RunningAppContext, task_details::TaskDetails},
};
use tokio::sync::RwLock;

use crate::app_state::SharedAppState;

pub struct Context {
    pub app_state: SharedAppState,
    pub task: Arc<RwLock<TaskDetails>>,
    pub app_data: AppData,
}

impl Context {
    pub async fn as_running_app_context(&self) -> RunningAppContext {
        RunningAppContext {
            task: self.task.read().await.clone(),
            app_data: self.app_data.clone(),
        }
    }

    pub fn create(app_state: SharedAppState, app_data: &AppData) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Context {
            app_state: app_state.clone(),
            app_data: app_data.clone(),
            task: Arc::new(RwLock::new(TaskDetails {
                app_name: Some(app_data.name.clone()),
                ..TaskDetails::default()
            })),
        }))
    }
}
