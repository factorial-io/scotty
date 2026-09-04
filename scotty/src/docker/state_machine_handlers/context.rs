use std::sync::Arc;

use scotty_core::{
    apps::app_data::AppData,
    tasks::{running_app_context::RunningAppContext, task_details::TaskDetails},
};
use tokio::sync::{watch, RwLock};

use crate::{
    app_state::SharedAppState,
    tasks::actor::{Snapshot, TaskHandle},
};

/// Shared by every handler of one operation, including nested state machines.
/// The context owns the task: when the last reference goes away (normal end,
/// error, or panic) without `task.terminate`, the task is failed automatically.
pub struct Context {
    pub app_state: SharedAppState,
    pub task: TaskHandle,
    task_snapshot: watch::Receiver<Snapshot>,
    pub app_data: AppData,
}

impl Context {
    pub async fn create(app_state: SharedAppState, app_data: &AppData) -> Arc<RwLock<Self>> {
        let (task, task_snapshot) = app_state
            .task_manager
            .create_task(TaskDetails {
                app_name: Some(app_data.name.clone()),
                ..TaskDetails::default()
            })
            .await;
        Arc::new(RwLock::new(Context {
            app_state,
            task,
            task_snapshot,
            app_data: app_data.clone(),
        }))
    }

    pub fn task_snapshot(&self) -> Snapshot {
        self.task_snapshot.borrow().clone()
    }

    pub fn as_running_app_context(&self) -> RunningAppContext {
        RunningAppContext {
            task: (*self.task_snapshot()).clone(),
            app_data: self.app_data.clone(),
        }
    }
}
