use std::sync::Arc;

use scotty_core::{
    apps::app_data::AppData,
    tasks::{running_app_context::RunningAppContext, task_details::TaskDetails},
};
use tokio::sync::watch;

use crate::{
    app_state::{AppLock, SharedAppState},
    tasks::actor::{Snapshot, TaskHandle},
};

/// Who started a task and, for an action run, the permission the action
/// required. Both decide who may see the task (`can_view_task`).
#[derive(Debug, Default)]
pub struct TaskAccess {
    pub requested_by: Option<String>,
    pub action_permission: Option<String>,
}

/// Shared by every step of one operation, including nested operations.
/// The context owns the task: when the last reference goes away (normal end,
/// error, or panic) without `task.terminate`, the task is failed automatically.
pub struct Context {
    pub app_state: SharedAppState,
    /// The operation's name, e.g. `rebuild` or `action:<name>`.
    pub operation: String,
    /// The app's operation lock, if the operation is exclusive. Declared
    /// before `task`: fields drop in order, so on a panic the app is unlocked
    /// before the task fails.
    lock: std::sync::Mutex<Option<AppLock>>,
    pub task: TaskHandle,
    task_snapshot: watch::Receiver<Snapshot>,
    pub app_data: AppData,
}

impl Context {
    pub async fn create(
        app_state: SharedAppState,
        app_data: &AppData,
        access: TaskAccess,
        operation: String,
        mut lock: Option<AppLock>,
    ) -> Arc<Self> {
        let (task, task_snapshot) = app_state
            .task_manager
            .create_task(TaskDetails {
                app_name: Some(app_data.name.clone()),
                requested_by: access.requested_by,
                action_permission: access.action_permission,
                ..TaskDetails::default()
            })
            .await;
        if let Some(lock) = &mut lock {
            lock.set_task(task.id(), app_state.clone());
        }
        Arc::new(Context {
            app_state,
            operation,
            lock: std::sync::Mutex::new(lock),
            task,
            task_snapshot,
            app_data: app_data.clone(),
        })
    }

    pub fn task_snapshot(&self) -> Snapshot {
        self.task_snapshot.borrow().clone()
    }

    /// Release the app's operation lock, if held. Idempotent.
    pub fn release_lock(&self) {
        let lock = self.lock.lock().unwrap_or_else(|e| e.into_inner()).take();
        drop(lock);
    }

    pub fn as_running_app_context(&self) -> RunningAppContext {
        RunningAppContext {
            task: (*self.task_snapshot()).clone(),
            app_data: AppData {
                operation: self.app_state.app_locks.current(&self.app_data.name),
                ..self.app_data.clone()
            },
        }
    }
}
