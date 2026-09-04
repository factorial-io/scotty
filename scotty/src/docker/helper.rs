use crate::{app_state::SharedAppState, state_machine::StateMachine};
use anyhow::anyhow;
use bollard::models::ContainerStateStatusEnum;
use bollard::query_parameters::InspectContainerOptions;
use scotty_core::apps::app_data::AppData;
use scotty_core::tasks::running_app_context::RunningAppContext;
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
    let context = Context::create(app_state, app).await;
    let running = {
        let ctx = context.read().await;
        ctx.task
            .writer()
            .status(format!("Starting app '{}'", app.name))
            .await;
        ctx.as_running_app_context()
    };
    // The join handle is not needed: the context owns the task handle, so a
    // panicking or erroring machine that never reaches its completion handler
    // fails the task when the context is dropped.
    let _handle = sm.spawn(context);
    Ok(running)
}

/// Flatten the result of awaiting a spawned state machine.
pub fn join_outcome(
    joined: Result<anyhow::Result<()>, tokio::task::JoinError>,
) -> anyhow::Result<()> {
    match joined {
        Ok(result) => result,
        Err(join) if join.is_panic() => Err(anyhow!("aborted unexpectedly (internal error)")),
        Err(join) => Err(anyhow!("was cancelled: {join}")),
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
    use crate::tasks::actor::{FailureKind, Outcome};
    use scotty_core::tasks::task_details::{State, TaskDetails};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::RwLock;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    enum S {
        Start,
        Fail,
        Done,
    }

    enum Behaviour {
        Panic,
        Error,
        TerminateThenError,
    }

    struct H(Behaviour);

    #[async_trait::async_trait]
    impl StateHandler<S, Context> for H {
        async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
            match self.0 {
                Behaviour::Panic => panic!("boom"),
                Behaviour::Error => anyhow::bail!("step failed"),
                Behaviour::TerminateThenError => {
                    context
                        .read()
                        .await
                        .task
                        .terminate(Outcome::failed(FailureKind::StepFailed, "handler said so"))
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
        assert_eq!(ctx.task.state, State::Running);

        let mut rx = app_state
            .task_manager
            .subscribe(&ctx.task.id)
            .await
            .unwrap();
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
    }

    #[tokio::test]
    async fn drop_does_not_overwrite_terminal_state() {
        let task = run(Behaviour::Error, Behaviour::TerminateThenError).await;
        assert_eq!(task.state, State::Failed);
        let statuses: Vec<_> = task
            .output
            .lines
            .iter()
            .filter(|l| l.content.contains("handler said so") || l.content.contains("aborted"))
            .collect();
        assert_eq!(statuses.len(), 1, "{statuses:?}");
        assert!(statuses[0].content.contains("handler said so"));
    }
}
