use crate::{app_state::SharedAppState, state_machine::StateMachine};
use anyhow::anyhow;
use bollard::models::ContainerStateStatusEnum;
use bollard::query_parameters::InspectContainerOptions;
use scotty_core::apps::app_data::AppData;
use scotty_core::tasks::running_app_context::RunningAppContext;
use scotty_core::tasks::task_details::State;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::error;

use super::state_machine_handlers::context::Context;

pub async fn run_sm<S>(
    app_state: SharedAppState,
    app: &AppData,
    sm: StateMachine<S, Context>,
) -> anyhow::Result<RunningAppContext>
where
    S: Copy
        + PartialEq
        + Eq
        + std::hash::Hash
        + 'static
        + std::marker::Sync
        + std::marker::Send
        + std::fmt::Debug,
{
    let context = Context::create(app_state, app);
    {
        let context = context.write().await;
        let task = context.task.clone();
        let task_id = task.read().await.id;
        context
            .app_state
            .task_manager
            .add_task(&task_id, task.clone(), None)
            .await;

        // Add initial status message for the app operation
        context
            .app_state
            .task_manager
            .add_task_status(&task_id, format!("Starting app '{}'", app.name))
            .await;
    }
    // Supervise the state machine instead of dropping its handle: a handler
    // panic, or an error-state handler that itself fails, would otherwise leave
    // the task Running forever since only TaskCompletionHandler terminates it.
    let handle = sm.spawn(context.clone());
    tokio::spawn(supervise(handle, context.clone()));

    // Return immediately with the context, task is running in background
    Ok(context.clone().read().await.as_running_app_context().await)
}

/// Flatten the outcome of awaiting a spawned state machine: a panic or a
/// cancelled task becomes an error like any handler failure.
pub fn join_outcome(
    joined: Result<anyhow::Result<()>, tokio::task::JoinError>,
) -> anyhow::Result<()> {
    match joined {
        Ok(result) => result,
        Err(join) if join.is_panic() => Err(anyhow!("aborted unexpectedly (internal error)")),
        Err(join) => Err(anyhow!("was cancelled: {join}")),
    }
}

/// Await a spawned top-level state machine and fail its task if it ended
/// without reaching a terminal state. Never overwrites a state the completion
/// handler already set.
async fn supervise(
    handle: tokio::task::JoinHandle<anyhow::Result<()>>,
    context: Arc<RwLock<Context>>,
) {
    let Err(cause) = join_outcome(handle.await) else {
        return;
    };
    let cause = format!("{cause:#}");

    let ctx = context.read().await;
    let app_name = &ctx.app_data.name;
    // True only if the supervisor had to fail the task itself, i.e. the
    // completion handler never ran.
    let supervisor_intervened = ctx
        .complete_task(
            State::Failed,
            format!("Operation for app '{app_name}' failed: {cause}"),
            true,
        )
        .await;
    if supervisor_intervened {
        let task_id = ctx.task.read().await.id;
        error!(%task_id, %app_name, %cause, "State machine ended without terminating its task");
    }
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
    use crate::state_machine::StateHandler;
    use scotty_core::tasks::task_details::TaskDetails;
    use std::time::Duration;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    enum S {
        Start,
        Fail,
        Done,
    }

    /// Handler that panics, errors, or completes the task itself before erroring.
    enum Behaviour {
        Panic,
        Error,
        CompleteThenError,
    }

    struct H(Behaviour);

    #[async_trait::async_trait]
    impl StateHandler<S, Context> for H {
        async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
            match self.0 {
                Behaviour::Panic => panic!("boom"),
                Behaviour::Error => anyhow::bail!("step failed"),
                Behaviour::CompleteThenError => {
                    context
                        .read()
                        .await
                        .complete_task(State::Failed, "handler said so".into(), true)
                        .await;
                    anyhow::bail!("after completion")
                }
            }
        }
    }

    async fn run(start: Behaviour, on_error: Behaviour) -> TaskDetails {
        let app_state = crate::api::test_utils::create_test_app_state_with_config(
            "tests/test_bearer_auth",
            None,
        )
        .await;
        let mut sm = StateMachine::new(S::Start, S::Done);
        sm.set_error_state(S::Fail);
        sm.add_handler(S::Start, Arc::new(H(start)));
        sm.add_handler(S::Fail, Arc::new(H(on_error)));
        let app = AppData {
            name: "supervised".into(),
            ..Default::default()
        };
        let ctx = run_sm(app_state.clone(), &app, sm).await.unwrap();

        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let task = app_state
                .task_manager
                .get_task_details(&ctx.task.id)
                .await
                .unwrap();
            if task.state != State::Running {
                return task;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "task never left Running"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    #[tokio::test]
    async fn complete_task_is_idempotent() {
        let app_state = crate::api::test_utils::create_test_app_state_with_config(
            "tests/test_bearer_auth",
            None,
        )
        .await;
        let app = AppData {
            name: "once".into(),
            ..Default::default()
        };
        let context = Context::create(app_state, &app);
        let ctx = context.read().await;
        assert!(ctx.complete_task(State::Failed, "first".into(), true).await);
        let first = ctx.task.read().await.clone();
        assert!(
            !ctx.complete_task(State::Finished, "second".into(), false)
                .await
        );
        let second = ctx.task.read().await.clone();
        assert_eq!(second.state, State::Failed);
        assert_eq!(second.finish_time, first.finish_time);
        assert_eq!(second.output.lines.len(), first.output.lines.len());
        assert!(!second
            .output
            .lines
            .iter()
            .any(|l| l.content.contains("second")));
    }

    #[tokio::test]
    async fn panicking_handler_fails_task() {
        let task = run(Behaviour::Panic, Behaviour::Error).await;
        assert_eq!(task.state, State::Failed);
        assert!(task.finish_time.is_some());
        assert!(task
            .output
            .lines
            .iter()
            .any(|l| l.content.contains("aborted unexpectedly")));
    }

    #[tokio::test]
    async fn failing_error_handler_still_fails_task() {
        let task = run(Behaviour::Error, Behaviour::Error).await;
        assert_eq!(task.state, State::Failed);
        assert!(task.finish_time.is_some());
        assert!(task
            .output
            .lines
            .iter()
            .any(|l| l.content.contains("step failed")));
    }

    #[tokio::test]
    async fn supervisor_does_not_overwrite_terminal_state() {
        let task = run(Behaviour::Error, Behaviour::CompleteThenError).await;
        assert_eq!(task.state, State::Failed);
        let statuses: Vec<_> = task
            .output
            .lines
            .iter()
            .filter(|l| {
                l.content.contains("handler said so") || l.content.contains("after completion")
            })
            .collect();
        assert_eq!(statuses.len(), 1, "{statuses:?}");
        assert!(statuses[0].content.contains("handler said so"));
    }
}
