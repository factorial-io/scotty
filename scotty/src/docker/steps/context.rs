use std::future::Future;
use std::sync::Arc;

use scotty_core::{
    apps::app_data::AppData,
    tasks::{running_app_context::RunningAppContext, task_details::TaskDetails},
};
use scotty_types::{StepProgress, StepState, TaskProgress};
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

/// A step of an operation, as shown in the task's progress.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Files,
    LoadBalancer,
    Login,
    Network,
    Pull,
    Build,
    Stop,
    Up,
    WaitForContainers,
    PostActions,
    RemoveContainers,
    RemoveNetwork,
    RemoveDirectory,
    /// One command of a custom action, see [`Step::command`].
    Command(String),
}

impl Step {
    /// The steps of a rebuild, in order. Create runs them as well.
    pub const REBUILD: [Step; 8] = [
        Step::Login,
        Step::Network,
        Step::Pull,
        Step::Build,
        Step::Stop,
        Step::Up,
        Step::WaitForContainers,
        Step::PostActions,
    ];

    /// The step that runs custom action `action` on `service`.
    pub fn command(action: &str, service: &str) -> Self {
        Step::Command(format!("Run {action} on {service}"))
    }

    pub fn name(&self) -> &str {
        match self {
            Step::Files => "Write files",
            Step::LoadBalancer => "Configure load balancer",
            Step::Login => "Log into registry",
            Step::Network => "Prepare network",
            Step::Pull => "Pull images",
            Step::Build => "Build images",
            Step::Stop => "Stop containers",
            Step::Up => "Start containers",
            Step::WaitForContainers => "Wait for containers",
            Step::PostActions => "Run post actions",
            Step::RemoveContainers => "Remove containers",
            Step::RemoveNetwork => "Remove network",
            Step::RemoveDirectory => "Remove app directory",
            Step::Command(name) => name,
        }
    }

    /// Static share of the step in its operation: the steps that usually
    /// take longest weigh most.
    fn weight(&self) -> u32 {
        match self {
            Step::Pull => 40,
            Step::Build => 30,
            Step::Up | Step::WaitForContainers => 10,
            _ => 2,
        }
    }
}

/// Initial progress for `steps`: the first one is running.
fn plan(steps: &[Step]) -> TaskProgress {
    TaskProgress {
        status: steps
            .first()
            .map(|s| s.name().to_string())
            .unwrap_or_default(),
        steps: steps
            .iter()
            .enumerate()
            .map(|(i, step)| StepProgress {
                name: step.name().to_string(),
                state: if i == 0 {
                    StepState::Running
                } else {
                    StepState::Pending
                },
                weight: step.weight(),
            })
            .collect(),
        ..TaskProgress::default()
    }
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
        steps: &[Step],
    ) -> Arc<Self> {
        let (task, task_snapshot) = app_state
            .task_manager
            .create_task(TaskDetails {
                app_name: Some(app_data.name.clone()),
                operation: Some(operation.clone()),
                requested_by: access.requested_by,
                action_permission: access.action_permission,
                progress: plan(steps),
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

    /// Run `work` as `step`: running, then done or failed. A step the
    /// operation did not declare is not shown.
    pub async fn step<T>(
        &self,
        step: &Step,
        work: impl Future<Output = anyhow::Result<T>>,
    ) -> anyhow::Result<T> {
        let writer = self.task.writer();
        writer.step(step.name(), StepState::Running).await;
        let result = work.await;
        let state = match result {
            Ok(_) => StepState::Done,
            Err(_) => StepState::Failed,
        };
        writer.step(step.name(), state).await;
        result
    }

    /// Mark `step` as not needed for this app.
    pub async fn skip_step(&self, step: &Step) {
        self.task
            .writer()
            .step(step.name(), StepState::Skipped)
            .await;
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
                ttl_expires_at: self.app_data.ttl_expiry(),
                blocked_paths: self
                    .app_data
                    .effective_blocked_paths(&self.app_state.settings.apps),
                ..self.app_data.clone()
            },
        }
    }
}
