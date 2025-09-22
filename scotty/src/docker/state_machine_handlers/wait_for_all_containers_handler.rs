use super::context::Context;
use crate::api::websocket::client::broadcast_message;
use crate::api::websocket::message::WebSocketMessage;
use crate::docker::helper::wait_for_containers_ready;
use crate::state_machine::StateHandler;
use anyhow::Context as _;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

/// A state machine handler that waits for all containers in an application to reach a ready state.
///
/// This handler finds all container IDs associated with the application's services and
/// waits until none of them are in a starting state (Created or Restarting).
#[derive(Debug)]
pub struct WaitForAllContainersHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    /// The next state to transition to after all containers are ready
    pub next_state: S,

    /// Optional timeout in seconds (defaults to 300 seconds if None)
    pub timeout_seconds: Option<u64>,
}

#[async_trait::async_trait]
impl<S> StateHandler<S, Context> for WaitForAllContainersHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    #[instrument(skip(context))]
    async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
        let (app_state, app_data, task_clone) = {
            let ctx = context.read().await;
            (
                ctx.app_state.clone(),
                ctx.app_data.clone(),
                ctx.task.clone(),
            )
        };

        debug!("Collecting container IDs for app {}", app_data.name);

        // Collecting all container IDs from app services
        let container_ids: Vec<String> = app_data
            .services
            .iter()
            .filter_map(|service| service.id.clone())
            .collect();

        if container_ids.is_empty() {
            warn!("No container IDs found for app {}", app_data.name);
            return Ok(self.next_state.clone());
        }

        debug!("Found {} containers to wait for", container_ids.len());

        // Add info messages to task output for client visibility
        let task_id = task_clone.read().await.id;
        app_state
            .task_manager
            .add_task_info(
                &task_id,
                format!(
                    "Waiting for {} containers to be ready: {:?}",
                    container_ids.len(),
                    container_ids
                ),
            )
            .await;

        info!("Waiting for containers to be ready: {:?}", container_ids);

        broadcast_message(
            &app_state,
            WebSocketMessage::TaskInfoUpdated(task_clone.read().await.clone()),
        )
        .await;

        // Wait for all containers to reach a non-starting state
        let container_states =
            wait_for_containers_ready(&app_state, container_ids, self.timeout_seconds)
                .await
                .context("Failed to wait for containers to be ready")?;

        // Add completion message to task output for client visibility
        app_state
            .task_manager
            .add_task_info(&task_id, "All containers are ready!".to_string())
            .await;

        info!("All containers have reached a ready state");
        debug!("Container states: {:?}", container_states);

        broadcast_message(
            &app_state,
            WebSocketMessage::TaskInfoUpdated(task_clone.read().await.clone()),
        )
        .await;

        // Return the next state
        Ok(self.next_state.clone())
    }
}
