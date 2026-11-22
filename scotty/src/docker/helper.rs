use std::future::Future;
use std::sync::Arc;

use crate::app_state::{AppLock, SharedAppState};
use crate::docker::find_apps::inspect_app;
use crate::tasks::actor::{FailureKind, Outcome};
use anyhow::{anyhow, Context as _};
use bollard::models::ContainerStateStatusEnum;
use bollard::query_parameters::InspectContainerOptions;
use scotty_core::apps::app_data::AppData;
use scotty_core::notification_types::Message;
use scotty_core::tasks::running_app_context::RunningAppContext;
use scotty_core::websocket::message::WebSocketMessage;
use tracing::{error, warn};

use super::steps::context::{Context, Step, TaskAccess};

/// How an operation relates to the app's operation lock.
enum LockSpec {
    /// Take the lock under this operation name; refused if it is held.
    Exclusive(String),
    /// Use a lock the caller already took (create takes it before reading the
    /// existing app from disk).
    Locked(AppLock),
    /// Read-only: neither takes nor waits for the lock.
    Shared(String),
}

/// What [`run_operation`] needs to know about an operation besides its steps.
pub struct OperationSpec {
    lock: LockSpec,
    changes_app_list: bool,
}

impl OperationSpec {
    fn new(lock: LockSpec) -> Self {
        Self {
            lock,
            changes_app_list: false,
        }
    }

    /// Take the app's lock under `name`; refused if it is held.
    pub fn exclusive(name: impl Into<String>) -> Self {
        Self::new(LockSpec::Exclusive(name.into()))
    }

    /// Use a lock the caller already took, under the lock's operation name.
    pub fn locked(lock: AppLock) -> Self {
        Self::new(LockSpec::Locked(lock))
    }

    /// Read-only: neither takes nor waits for the lock.
    pub fn shared(name: impl Into<String>) -> Self {
        Self::new(LockSpec::Shared(name.into()))
    }

    /// The operation adds or removes the app, so clients refetch the app list
    /// when it ends.
    pub fn changing_app_list(mut self) -> Self {
        self.changes_app_list = true;
        self
    }
}

/// The error an operation's future is replaced with when it runs longer than
/// `apps.max_operation_duration`.
#[derive(Debug, thiserror::Error)]
#[error("Operation {operation} for app '{app}' aborted after exceeding the maximum operation duration of {limit}")]
struct OperationTimedOut {
    operation: String,
    app: String,
    limit: String,
}

/// Run an app operation in the background and own its task.
///
/// Creates the [`Context`] (and with it the task), returns the
/// [`RunningAppContext`] immediately, and spawns `op`. When `op` returns the
/// app data is refreshed and the task is terminated exactly once: a refresh
/// error wins over success, a step error fails the task with its cause, and
/// `notification` is sent only on success. A panic inside `op` is covered by
/// `TaskHandle::Drop`, which fails the task when the context is dropped.
/// `steps` are the steps `op` runs, in order; the task shows them as its
/// progress from the start.
pub async fn run_operation<F, Fut>(
    app_state: SharedAppState,
    app: &AppData,
    operation: OperationSpec,
    steps: &[Step],
    notification: Option<Message>,
    op: F,
) -> anyhow::Result<RunningAppContext>
where
    F: FnOnce(Arc<Context>) -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    run_operation_requested_by(
        app_state,
        app,
        operation,
        steps,
        TaskAccess::default(),
        notification,
        op,
    )
    .await
}

/// Like [`run_operation`], but records who started the task, so that user can
/// follow it before the app is assigned to a scope (app creation), and for
/// action runs the permission the action required (task visibility).
///
/// An exclusive operation takes the app's lock before the task is created: a
/// busy app is refused with [`AppError::OperationInProgress`] and no task.
/// The lock is held until the operation has ended, at most
/// `apps.max_operation_duration`.
pub async fn run_operation_requested_by<F, Fut>(
    app_state: SharedAppState,
    app: &AppData,
    operation: OperationSpec,
    steps: &[Step],
    access: TaskAccess,
    notification: Option<Message>,
    op: F,
) -> anyhow::Result<RunningAppContext>
where
    F: FnOnce(Arc<Context>) -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let (name, lock) = match operation.lock {
        LockSpec::Exclusive(name) => {
            let lock = app_state.app_locks.lock(&app.name, &name)?;
            (name, Some(lock))
        }
        LockSpec::Locked(lock) => (lock.operation().to_string(), Some(lock)),
        LockSpec::Shared(name) => (name, None),
    };
    let locked = lock.is_some();
    let ctx = Context::create(app_state.clone(), app, access, name, lock, steps).await;
    let changes_app_list = operation.changes_app_list;
    ctx.task
        .writer()
        .status(format!("Starting app '{}'", app.name))
        .await;
    if locked {
        app_state.push_app_info(&app.name).await;
    }
    let running = ctx.as_running_app_context();
    let limit = app_state.settings.apps.max_operation_duration.clone();
    crate::metrics::spawn_instrumented(async move {
        // Cancelling `op` drops its running subprocess, which kills it.
        let result = match tokio::time::timeout(limit.clone().into(), op(ctx.clone())).await {
            Ok(result) => result,
            Err(_) => {
                warn!(
                    app = %ctx.app_data.name,
                    operation = %ctx.operation,
                    task_id = %ctx.task.id(),
                    limit = %limit,
                    "Operation exceeded the maximum operation duration; aborted"
                );
                Err(OperationTimedOut {
                    operation: ctx.operation.clone(),
                    app: ctx.app_data.name.clone(),
                    limit: limit.to_string(),
                }
                .into())
            }
        };
        finish_operation(&ctx, result, notification, changes_app_list).await;
    });
    Ok(running)
}

/// Refresh app data, release the app's lock, terminate the task once, notify
/// on success, and tell every client to refetch the app list if
/// `changes_app_list`. The lock is released before the task turns terminal, so
/// a client that waits for the task can start the next operation right away.
async fn finish_operation(
    ctx: &Context,
    result: anyhow::Result<()>,
    notification: Option<Message>,
    changes_app_list: bool,
) {
    finish_operation_after(
        ctx,
        result,
        notification,
        changes_app_list,
        refresh_app_data(ctx),
    )
    .await
}

/// How long the refresh after an operation may take. Docker can hang (e.g.
/// right after an operation was aborted); the lock and the task must not
/// hang with it.
const REFRESH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// [`finish_operation`] with the refresh passed in, so tests can stall it.
async fn finish_operation_after(
    ctx: &Context,
    result: anyhow::Result<()>,
    notification: Option<Message>,
    changes_app_list: bool,
    refresh: impl Future<Output = anyhow::Result<()>>,
) {
    let app_name = &ctx.app_data.name;
    let refreshed = match tokio::time::timeout(REFRESH_TIMEOUT, refresh).await {
        Ok(refreshed) => refreshed,
        Err(_) => {
            warn!(
                app = %app_name,
                operation = %ctx.operation,
                "Refreshing app data after the operation timed out; releasing the app anyway"
            );
            Err(anyhow!("timed out after {}s", REFRESH_TIMEOUT.as_secs()))
        }
    };
    let outcome = match (result, refreshed) {
        (Err(e), _) if e.is::<OperationTimedOut>() => {
            Outcome::failed(FailureKind::TimedOut, e.to_string())
        }
        (_, Err(e)) => Outcome::failed(
            FailureKind::RefreshFailed,
            format!("Operation for app '{app_name}' ended, but refreshing app data failed: {e:#}"),
        ),
        (Err(e), Ok(())) => {
            error!(app = %app_name, error = %format!("{e:#}"), "Operation failed");
            Outcome::failed(
                FailureKind::StepFailed,
                format!("Operation failed for app '{app_name}': {e:#}"),
            )
        }
        (Ok(()), Ok(())) => Outcome::finished(format!(
            "Successfully completed operation for app '{app_name}'"
        )),
    };
    let finished = matches!(outcome, Outcome::Finished { .. });
    ctx.release_lock();
    ctx.task.terminate(outcome).await;
    if changes_app_list {
        ctx.app_state
            .messenger
            .broadcast_to_all(WebSocketMessage::AppListUpdated)
            .await;
    }

    if !finished {
        return;
    }
    if let (Some(settings), Some(notification)) = (&ctx.app_data.settings, notification) {
        if let Err(err) =
            crate::notification::notify::notify(&ctx.app_state, &settings.notify, &notification)
                .await
        {
            error!(app = %app_name, "Failed to send notification: {err:?}");
        }
    }
}

/// Re-inspect the app after an operation. No compose file means the app was
/// removed (destroy deletes the directory before completing): nothing to
/// refresh, not a failure.
async fn refresh_app_data(ctx: &Context) -> anyhow::Result<()> {
    let docker_compose_path = std::path::PathBuf::from(&ctx.app_data.docker_compose_path);
    if !docker_compose_path.exists() {
        tracing::debug!(
            "Compose file {} is gone, skipping app data refresh",
            docker_compose_path.display()
        );
        return Ok(());
    }
    let app_data = inspect_app(&ctx.app_state, &docker_compose_path)
        .await
        .context("Refreshing app data after task completion failed")?;
    ctx.app_state.apps.update_app(app_data).await?;
    Ok(())
}

/// Wait for all containers to reach a non-starting state.
///
/// This function waits until all the specified containers are either running successfully
/// or have failed (not in 'created' or 'restarting' state).
///
/// # Arguments
///
/// * `app_state` - The shared application state containing the Docker client
/// * `container_ids` - A vector of Docker container IDs to monitor
/// * `timeout_seconds` - Optional timeout in seconds (defaults to 300 seconds)
///
/// # Returns
///
/// * `anyhow::Result<Vec<(String, ContainerStateStatusEnum)>>` - Container IDs and their states when they're all ready or an error
///
/// # Example
///
/// ```no_run
/// use scotty::docker::helper::wait_for_containers_ready;
/// use scotty::app_state::SharedAppState;
///
/// async fn example(app_state: &SharedAppState) -> anyhow::Result<()> {
///     let container_ids = vec!["container1".to_string(), "container2".to_string()];
///     let container_states = wait_for_containers_ready(app_state, container_ids, Some(60)).await?;
///
///     for (container_id, status) in container_states {
///         println!("Container {} is in state: {:?}", container_id, status);
///     }
///
///     Ok(())
/// }
/// ```
pub async fn wait_for_containers_ready(
    app_state: &SharedAppState,
    container_ids: Vec<String>,
    timeout_seconds: Option<u64>,
) -> anyhow::Result<Vec<(String, ContainerStateStatusEnum)>> {
    // Default timeout of 300 seconds (5 minutes) if not specified
    let timeout = timeout_seconds.unwrap_or(300);
    let timeout_duration = tokio::time::Duration::from_secs(timeout);

    // Create a timeout for the entire operation
    let result = tokio::time::timeout(timeout_duration, async {
        let mut all_ready = false;
        let mut current_states = Vec::new();

        // Keep checking until all containers are ready or in error state
        while !all_ready {
            current_states.clear();

            // Check each container's status
            for container_id in &container_ids {
                match app_state
                    .docker
                    .inspect_container(container_id, None::<InspectContainerOptions>)
                    .await
                {
                    Ok(container_info) => {
                        if let Some(state) = container_info.state {
                            if let Some(status) = state.status {
                                // Store the container ID and its status directly
                                current_states.push((container_id.clone(), status));
                            }
                        }
                    }
                    Err(e) => {
                        // Log the error but continue with other containers
                        error!("Failed to inspect container {}: {}", container_id, e);
                        // Add a container in error state
                        current_states.push((container_id.clone(), ContainerStateStatusEnum::DEAD));
                    }
                }
            }

            // Check if any container is still in a starting state
            let starting_containers = current_states
                .iter()
                .filter(|(_, status)| {
                    *status == ContainerStateStatusEnum::CREATED
                        || *status == ContainerStateStatusEnum::RESTARTING
                })
                .count();

            if starting_containers == 0 {
                // All containers are either running or in an error state
                all_ready = true;
            } else {
                // Wait a bit before checking again
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }

        Ok(current_states)
    })
    .await;

    match result {
        Ok(r) => r,
        Err(_) => Err(anyhow!("Timeout waiting for containers to be ready")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::error::AppError;
    use scotty_core::tasks::task_details::{State, TaskDetails};
    use scotty_types::StepState;
    use std::time::Duration;

    async fn test_app_state() -> SharedAppState {
        crate::api::test_utils::create_test_app_state_with_config("tests/test_bearer_auth", None)
            .await
    }

    async fn wait_terminal(app_state: &SharedAppState, id: uuid::Uuid) -> TaskDetails {
        let mut rx = app_state.task_manager.subscribe(&id).await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let task = rx.borrow_and_update().clone();
                if task.state != State::Running {
                    return (*task).clone();
                }
                rx.changed().await.unwrap();
            }
        })
        .await
        .expect("task never left Running")
    }

    fn statuses(task: &TaskDetails) -> Vec<String> {
        task.output
            .lines
            .iter()
            .map(|l| l.content.clone())
            .collect()
    }

    /// An app whose compose file does not exist: refresh is skipped.
    fn gone_app(name: &str) -> AppData {
        AppData {
            name: name.into(),
            docker_compose_path: "/nonexistent/scotty-test/docker-compose.yml".into(),
            root_directory: "/nonexistent/scotty-test".into(),
            ..Default::default()
        }
    }

    async fn run<Fut>(
        app: AppData,
        op: impl FnOnce(Arc<Context>) -> Fut + Send + 'static,
    ) -> TaskDetails
    where
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let app_state = test_app_state().await;
        let ctx = run_operation(
            app_state.clone(),
            &app,
            OperationSpec::exclusive("run"),
            &[],
            None,
            op,
        )
        .await
        .unwrap();
        assert_eq!(ctx.task.state, State::Running);
        let task = wait_terminal(&app_state, ctx.task.id).await;
        // However the operation ended, the app is free once its task is
        // terminal.
        assert!(app_state.app_locks.current(&app.name).is_none());
        task
    }

    /// The app is busy while the operation runs: a second one is refused
    /// with the holder and creates no task; once the first task is terminal,
    /// the next one is accepted at once.
    #[tokio::test]
    async fn second_operation_is_refused_until_the_first_ended() {
        let app_state = test_app_state().await;
        let app = gone_app("busy");
        let (release, released) = tokio::sync::oneshot::channel::<()>();
        let first = run_operation(
            app_state.clone(),
            &app,
            OperationSpec::exclusive("run"),
            &[],
            None,
            |_| async move {
                let _ = released.await;
                Ok(())
            },
        )
        .await
        .unwrap();
        let operation = first
            .app_data
            .operation
            .clone()
            .expect("response shows the lock");
        assert_eq!(operation.operation, "run");
        assert_eq!(operation.task_id, Some(first.task.id));

        let refused = run_operation(
            app_state.clone(),
            &app,
            OperationSpec::exclusive("run"),
            &[],
            None,
            |_| async { Ok(()) },
        )
        .await
        .unwrap_err();
        match AppError::from(refused) {
            AppError::OperationInProgress { operation, .. } => {
                assert_eq!(operation.operation, "run");
                assert_eq!(operation.task_id, Some(first.task.id));
            }
            other => panic!("expected OperationInProgress, got {other:?}"),
        }
        assert_eq!(app_state.task_manager.get_task_list().await.len(), 1);

        release.send(()).unwrap();
        let task = wait_terminal(&app_state, first.task.id).await;
        assert_eq!(task.state, State::Finished);
        let next = run_operation(
            app_state.clone(),
            &app,
            OperationSpec::exclusive("run"),
            &[],
            None,
            |_| async { Ok(()) },
        )
        .await;
        assert!(next.is_ok(), "next run is accepted right away");
    }

    /// Read-only operations neither take nor wait for the lock.
    #[tokio::test]
    async fn shared_operation_runs_during_an_exclusive_one() {
        let app_state = test_app_state().await;
        let app = gone_app("shared");
        let rebuild = app_state.app_locks.lock(&app.name, "rebuild").unwrap();
        for _ in 0..2 {
            let running = run_operation(
                app_state.clone(),
                &app,
                OperationSpec::shared("action:status"),
                &[],
                None,
                |_| std::future::pending(),
            )
            .await
            .unwrap();
            assert_eq!(running.app_data.operation.unwrap().operation, "rebuild");
        }
        drop(rebuild);
        let _read = run_operation(
            app_state.clone(),
            &app,
            OperationSpec::shared("action:status"),
            &[],
            None,
            |_| std::future::pending(),
        )
        .await
        .unwrap();
        assert!(
            app_state.app_locks.current(&app.name).is_none(),
            "a read-only action is not the app's operation"
        );
    }

    /// An operation over the limit is aborted: its process is killed, the
    /// task fails naming the limit, the app is unlocked and viewers hear of
    /// it.
    #[tokio::test]
    async fn operation_over_the_limit_is_aborted() {
        let settings: crate::settings::config::Settings = config::Config::builder()
            .add_source(config::File::with_name("tests/test_bearer_auth"))
            .set_override("apps.max_operation_duration", "1s")
            .unwrap()
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap();
        let app_state =
            crate::api::test_utils::create_test_app_state_with_settings(settings, None).await;
        let dir = tempfile::tempdir().unwrap();
        let pid_file = dir.path().join("pid");
        let app = AppData {
            name: "slow".into(),
            // Never written: the refresh is skipped.
            docker_compose_path: dir
                .path()
                .join("docker-compose.yml")
                .to_string_lossy()
                .into(),
            root_directory: dir.path().to_string_lossy().into(),
            ..Default::default()
        };
        app_state
            .auth_service
            .set_app_scopes("slow", vec!["client-a".to_string()])
            .await
            .unwrap();
        let mut viewer = connect(&app_state, "identifier:client-a").await;
        let mut outsider = connect(&app_state, "someone@example.com").await;

        let script = format!("echo $$ > {}; exec sleep 30", pid_file.display());
        let compose_path = std::path::PathBuf::from(&app.docker_compose_path);
        let running = run_operation(
            app_state.clone(),
            &app,
            OperationSpec::exclusive("rebuild"),
            &[],
            None,
            move |ctx| async move {
                crate::docker::steps::run_task_and_wait::run_task_and_wait(
                    &ctx,
                    &compose_path,
                    "sh",
                    &["-c", &script],
                    &Default::default(),
                    None,
                    "sleep",
                    crate::tasks::manager::OutputMode::Plain,
                )
                .await
            },
        )
        .await
        .unwrap();

        let task = wait_terminal(&app_state, running.task.id).await;
        assert_eq!(task.state, State::Failed);
        assert_eq!(
            statuses(&task).last().unwrap(),
            "Operation rebuild for app 'slow' aborted after exceeding the maximum \
             operation duration of 1s"
        );
        assert!(app_state.app_locks.current("slow").is_none());

        let pid = std::fs::read_to_string(&pid_file).unwrap();
        let alive = || {
            std::process::Command::new("kill")
                .args(["-0", pid.trim()])
                .status()
                .unwrap()
                .success()
        };
        tokio::time::timeout(Duration::from_secs(5), async {
            while alive() {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("the process outlived its operation");

        // Start and release are pushed to the viewer only.
        assert_eq!(app_info_updates(&mut viewer, "slow").await, 2);
        assert_eq!(app_info_updates(&mut outsider, "slow").await, 0);
    }

    /// The TTL checker's destroy changes the app list like a user's destroy.
    #[tokio::test]
    async fn ttl_destroy_broadcasts_the_app_list_change() {
        let app_state = test_app_state().await;
        let mut client = connect(&app_state, "someone@example.com").await;
        let running = run_operation(
            app_state.clone(),
            &gone_app("expired"),
            crate::docker::destroy_app::operation("ttl-destroy"),
            &[],
            None,
            |_| async { Ok(()) },
        )
        .await
        .unwrap();
        wait_terminal(&app_state, running.task.id).await;

        let expected = serde_json::to_string(&WebSocketMessage::AppListUpdated).unwrap();
        let mut seen = false;
        while let Ok(Ok(message)) =
            tokio::time::timeout(Duration::from_millis(200), client.recv()).await
        {
            seen |= matches!(message, axum::extract::ws::Message::Text(ref t) if t.as_str() == expected);
        }
        assert!(seen, "AppListUpdated was not broadcast");
    }

    /// The lock is keyed by the app's slug, but start and release are
    /// announced under the app's own name, the one its scopes are known by.
    /// A directory placed by hand keeps a name that differs from its slug.
    #[tokio::test]
    async fn lock_pushes_under_the_name_the_scopes_use() {
        let app_state = test_app_state().await;
        app_state
            .auth_service
            .set_app_scopes("simple_nginx", vec!["client-a".to_string()])
            .await
            .unwrap();
        let mut viewer = connect(&app_state, "identifier:client-a").await;
        let running = run_operation(
            app_state.clone(),
            &gone_app("simple_nginx"),
            OperationSpec::exclusive("run"),
            &[],
            None,
            |_| async { Ok(()) },
        )
        .await
        .unwrap();
        wait_terminal(&app_state, running.task.id).await;
        assert_eq!(app_info_updates(&mut viewer, "simple_nginx").await, 2);
    }

    /// A refresh that never returns (stuck Docker daemon) still lets the
    /// app's lock go and the task end.
    #[tokio::test(start_paused = true)]
    async fn a_stuck_refresh_does_not_keep_the_lock() {
        let app_state = test_app_state().await;
        let app = gone_app("stuck");
        let lock = app_state.app_locks.lock(&app.name, "rebuild").unwrap();
        let ctx = Context::create(
            app_state.clone(),
            &app,
            TaskAccess::default(),
            "rebuild".into(),
            Some(lock),
            &[],
        )
        .await;
        tokio::time::timeout(
            REFRESH_TIMEOUT * 2,
            finish_operation_after(&ctx, Ok(()), None, false, std::future::pending()),
        )
        .await
        .expect("finishing waited for the stuck refresh");
        assert!(app_state.app_locks.current(&app.name).is_none());
        let task = wait_terminal(&app_state, ctx.task.id()).await;
        assert_eq!(task.state, State::Failed);
        assert!(
            statuses(&task).last().unwrap().contains("timed out"),
            "{:?}",
            statuses(&task)
        );
    }

    async fn connect(
        app_state: &SharedAppState,
        user: &str,
    ) -> tokio::sync::broadcast::Receiver<axum::extract::ws::Message> {
        let (tx, rx) = tokio::sync::broadcast::channel(100);
        let id = uuid::Uuid::new_v4();
        app_state
            .messenger
            .add_client(id, crate::app_state::WebSocketClient::new(tx))
            .await;
        let user = crate::api::basic_auth::CurrentUser {
            email: user.to_string(),
            name: user.to_string(),
            picture: None,
            access_token: None,
        };
        app_state
            .messenger
            .authenticate_client(id, user)
            .await
            .unwrap();
        rx
    }

    /// `AppInfoUpdated(app)` messages received, waiting briefly for pushes
    /// spawned on release.
    async fn app_info_updates(
        rx: &mut tokio::sync::broadcast::Receiver<axum::extract::ws::Message>,
        app: &str,
    ) -> usize {
        let expected =
            serde_json::to_string(&WebSocketMessage::AppInfoUpdated(app.into())).unwrap();
        let mut count = 0;
        while let Ok(Ok(message)) =
            tokio::time::timeout(Duration::from_millis(200), rx.recv()).await
        {
            if matches!(message, axum::extract::ws::Message::Text(ref t) if t.as_str() == expected)
            {
                count += 1;
            }
        }
        count
    }

    async fn wait_for(
        rx: &mut tokio::sync::watch::Receiver<crate::tasks::actor::Snapshot>,
        pred: impl Fn(&TaskDetails) -> bool,
    ) -> TaskDetails {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let task = rx.borrow_and_update().clone();
                if pred(&task) {
                    return (*task).clone();
                }
                rx.changed().await.unwrap();
            }
        })
        .await
        .expect("task never satisfied predicate")
    }

    /// Runs the rebuild plan with stubbed steps: `fail_at` fails, the login
    /// is skipped (no registry). Returns the step states and the fraction
    /// after each step, and the terminal task.
    async fn stubbed_rebuild(fail_at: Option<Step>) -> (Vec<(Vec<StepState>, f32)>, TaskDetails) {
        let app_state = test_app_state().await;
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let (state, seen_op) = (app_state.clone(), seen.clone());
        let running = run_operation(
            app_state.clone(),
            &gone_app("stubbed"),
            OperationSpec::exclusive("rebuild"),
            &Step::REBUILD,
            None,
            move |ctx| async move {
                let mut rx = state.task_manager.subscribe(&ctx.task.id()).await.unwrap();
                for (i, step) in Step::REBUILD.iter().enumerate() {
                    let result = match step {
                        Step::Login => {
                            ctx.skip_step(step).await;
                            Ok(())
                        }
                        _ if Some(step) == fail_at.as_ref() => {
                            ctx.step(step, async { anyhow::bail!("stub failed") }).await
                        }
                        _ => ctx.step(step, async { Ok(()) }).await,
                    };
                    let task = wait_for(&mut rx, |t| {
                        !matches!(
                            t.progress.steps[i].state,
                            StepState::Pending | StepState::Running
                        )
                    })
                    .await;
                    let states = task.progress.steps.iter().map(|s| s.state).collect();
                    seen_op
                        .lock()
                        .unwrap()
                        .push((states, task.progress.fraction));
                    result?;
                }
                Ok(())
            },
        )
        .await
        .unwrap();

        // Declared up front: the first step runs, the others are pending.
        let progress = &running.task.progress;
        let names: Vec<_> = progress.steps.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Log into registry",
                "Prepare network",
                "Pull images",
                "Build images",
                "Stop containers",
                "Start containers",
                "Wait for containers",
                "Run post actions"
            ]
        );
        assert_eq!(progress.steps[0].state, StepState::Running);
        assert!(progress.steps[1..]
            .iter()
            .all(|s| s.state == StepState::Pending));
        assert_eq!(progress.fraction, 0.0);
        assert_eq!(progress.status, "Log into registry");

        let task = wait_terminal(&app_state, running.task.id).await;
        let seen = seen.lock().unwrap().clone();
        (seen, task)
    }

    #[tokio::test]
    async fn rebuild_reports_step_states_and_fraction() {
        use StepState::*;
        let (seen, task) = stubbed_rebuild(None).await;
        // Weights: pull 40, build 30, up 10, wait 10, every other step 2.
        let expected = [2, 4, 44, 74, 76, 86, 96, 98];
        for (i, ((states, fraction), done)) in seen.iter().zip(expected).enumerate() {
            let want: Vec<StepState> = (0..8)
                .map(|j| match j {
                    0 => Skipped,
                    j if j <= i => Done,
                    _ => Pending,
                })
                .collect();
            assert_eq!(states, &want, "after step {i}");
            assert!(
                (fraction - done as f32 / 98.0).abs() < 1e-6,
                "after step {i}: {fraction}"
            );
        }
        assert_eq!(task.state, State::Finished);
        assert_eq!(task.progress.fraction, 1.0);
    }

    /// Runs `name` with two subprocess steps and returns the task as seen
    /// after each step started, plus the terminal task.
    async fn two_commands(name: &str) -> (Vec<TaskDetails>, TaskDetails) {
        let app_state = test_app_state().await;
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let (state, seen_op) = (app_state.clone(), seen.clone());
        let running = run_operation(
            app_state.clone(),
            &gone_app("named"),
            OperationSpec::exclusive(name),
            &[],
            None,
            move |ctx| async move {
                let mut rx = state.task_manager.subscribe(&ctx.task.id()).await.unwrap();
                for command in ["docker compose pull", "docker compose build"] {
                    ctx.task.writer().step_started(command).await;
                    let task = wait_for(&mut rx, |t| t.command == command).await;
                    seen_op.lock().unwrap().push(task);
                }
                Ok(())
            },
        )
        .await
        .unwrap();
        assert_eq!(running.task.operation.as_deref(), Some(name));
        let task = wait_terminal(&app_state, running.task.id).await;
        let seen = seen.lock().unwrap().clone();
        (seen, task)
    }

    #[tokio::test]
    async fn operation_stays_while_command_follows_the_steps() {
        let (seen, task) = two_commands("rebuild").await;
        let pairs: Vec<_> = seen
            .iter()
            .map(|t| (t.operation.as_deref(), t.command.as_str()))
            .collect();
        assert_eq!(
            pairs,
            [
                (Some("rebuild"), "docker compose pull"),
                (Some("rebuild"), "docker compose build")
            ]
        );
        assert_eq!(task.operation.as_deref(), Some("rebuild"));
    }

    #[tokio::test]
    async fn custom_action_task_carries_its_action_name() {
        let (seen, task) = two_commands("action:drush:uli").await;
        assert!(seen
            .iter()
            .all(|t| t.operation.as_deref() == Some("action:drush:uli")));
        assert_eq!(task.operation.as_deref(), Some("action:drush:uli"));
    }

    #[tokio::test]
    async fn failed_step_leaves_later_steps_pending() {
        use StepState::*;
        let (seen, task) = stubbed_rebuild(Some(Step::Build)).await;
        let (states, fraction) = seen.last().unwrap();
        assert_eq!(
            states,
            &[Skipped, Done, Done, Failed, Pending, Pending, Pending, Pending]
        );
        assert_eq!(task.state, State::Failed);
        assert_eq!(task.progress.fraction, *fraction);
        assert_eq!(
            task.progress
                .steps
                .iter()
                .map(|s| s.state)
                .collect::<Vec<_>>(),
            *states
        );
    }

    #[tokio::test]
    async fn success_ends_finished_with_success_line() {
        let task = run(gone_app("ok"), |ctx| async move {
            ctx.task.writer().status("step one").await;
            Ok(())
        })
        .await;
        assert_eq!(task.state, State::Finished);
        assert!(task.finish_time.is_some());
        assert_eq!(
            statuses(&task),
            [
                "Starting app 'ok'",
                "step one",
                "Successfully completed operation for app 'ok'"
            ]
        );
    }

    #[tokio::test]
    async fn step_error_ends_failed_with_cause() {
        let task = run(gone_app("broken"), |_| async {
            anyhow::bail!("step failed")
        })
        .await;
        assert_eq!(task.state, State::Failed);
        assert!(task.finish_time.is_some());
        let last = statuses(&task).pop().unwrap();
        assert_eq!(last, "Operation failed for app 'broken': step failed");
    }

    #[tokio::test]
    async fn panicking_operation_ends_failed() {
        let task = run(gone_app("panic"), |_| async { panic!("boom") }).await;
        assert_eq!(task.state, State::Failed);
        assert!(statuses(&task)
            .last()
            .unwrap()
            .contains("aborted unexpectedly"));
    }

    /// A compose file outside the apps root cannot be inspected; the task
    /// must still terminate and report the refresh failure.
    #[tokio::test]
    async fn failed_refresh_ends_failed() {
        let dir = std::env::temp_dir().join(format!("scotty-refresh-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let compose = dir.join("docker-compose.yml");
        std::fs::write(&compose, "services: {}\n").unwrap();

        let task = run(
            AppData {
                name: "unrefreshable".into(),
                docker_compose_path: compose.to_string_lossy().into_owned(),
                root_directory: dir.to_string_lossy().into_owned(),
                ..Default::default()
            },
            |_| async { Ok(()) },
        )
        .await;
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(task.state, State::Failed);
        assert!(statuses(&task)
            .last()
            .unwrap()
            .contains("refreshing app data failed"));
    }
}
