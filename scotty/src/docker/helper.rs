use crate::{app_state::SharedAppState, state_machine::StateMachine};
use anyhow::anyhow;
use bollard::models::ContainerStateStatusEnum;
use bollard::query_parameters::InspectContainerOptions;
use scotty_core::apps::app_data::AppData;
use scotty_core::tasks::running_app_context::RunningAppContext;
use scotty_core::tasks::task_details::State;
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
    let handle = sm.spawn(context.clone()).await;

    // Gracefully handle both errors and panics from state machine execution
    match handle.await {
        Ok(Ok(())) => {
            // State machine completed successfully
        }
        Ok(Err(e)) => {
            // State machine returned an error
            // The error handler should have already marked task as Failed and set finish_time
            error!("State machine failed for app '{}': {}", app.name, e);

            // Add additional error context to task output
            let task_id = context.read().await.task.read().await.id;
            context
                .read()
                .await
                .app_state
                .task_manager
                .add_task_status_error(&task_id, format!("Operation failed: {}", e))
                .await;
        }
        Err(join_err) => {
            // Task panicked - this bypassed error handlers, so we need to manually mark task as Failed
            error!(
                "State machine task panicked for app '{}': {}",
                app.name, join_err
            );

            // Get task Arc first to avoid borrow checker issues
            let (task_id, task_arc) = {
                let ctx = context.read().await;
                let task = ctx.task.clone();
                let id = task.read().await.id;
                (id, task)
            };

            // Manually mark task as Failed since panic bypassed error handlers
            {
                let mut task_details = task_arc.write().await;
                task_details.state = State::Failed;
                task_details.finish_time = Some(chrono::Utc::now());
                task_details.output_collection_active = false;
            }

            context
                .read()
                .await
                .app_state
                .task_manager
                .add_task_status_error(&task_id, "Internal error: operation panicked".to_string())
                .await;
        }
    }

    Ok(context.clone().read().await.as_running_app_context().await)
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
