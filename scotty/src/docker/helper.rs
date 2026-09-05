use std::future::Future;
use std::sync::Arc;

use crate::app_state::SharedAppState;
use crate::docker::find_apps::inspect_app;
use crate::tasks::actor::{FailureKind, Outcome};
use anyhow::{anyhow, Context as _};
use bollard::models::ContainerStateStatusEnum;
use bollard::query_parameters::InspectContainerOptions;
use scotty_core::apps::app_data::AppData;
use scotty_core::notification_types::Message;
use scotty_core::tasks::running_app_context::RunningAppContext;
use tracing::error;

use super::steps::context::Context;

/// Run an app operation in the background and own its task.
///
/// Creates the [`Context`] (and with it the task), returns the
/// [`RunningAppContext`] immediately, and spawns `op`. When `op` returns the
/// app data is refreshed and the task is terminated exactly once: a refresh
/// error wins over success, a step error fails the task with its cause, and
/// `notification` is sent only on success. A panic inside `op` is covered by
/// `TaskHandle::Drop`, which fails the task when the context is dropped.
pub async fn run_operation<F, Fut>(
    app_state: SharedAppState,
    app: &AppData,
    notification: Option<Message>,
    op: F,
) -> anyhow::Result<RunningAppContext>
where
    F: FnOnce(Arc<Context>) -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let ctx = Context::create(app_state, app).await;
    ctx.task
        .writer()
        .status(format!("Starting app '{}'", app.name))
        .await;
    let running = ctx.as_running_app_context();
    crate::metrics::spawn_instrumented(async move {
        let result = op(ctx.clone()).await;
        finish_operation(&ctx, result, notification).await;
    });
    Ok(running)
}

/// Refresh app data, terminate the task once, notify on success.
pub async fn finish_operation(
    ctx: &Context,
    result: anyhow::Result<()>,
    notification: Option<Message>,
) {
    let app_name = &ctx.app_data.name;
    let outcome = match (result, refresh_app_data(ctx).await) {
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
    ctx.task.terminate(outcome).await;

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
    use scotty_core::tasks::task_details::{State, TaskDetails};
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
        let ctx = run_operation(app_state.clone(), &app, None, op)
            .await
            .unwrap();
        assert_eq!(ctx.task.state, State::Running);
        wait_terminal(&app_state, ctx.task.id).await
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
