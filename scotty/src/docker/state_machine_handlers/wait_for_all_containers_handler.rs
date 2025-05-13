use super::context::Context;
use crate::api::message::WebSocketMessage;
use crate::api::ws::broadcast_message;
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
        let context_read = context.read().await;
        let app_state = &context_read.app_state;
        let app_data = &context_read.app_data;

        // Update task status
        let task_clone = context_read.task.clone();
        {
            let mut task = task_clone.write().await;
            task.println(format!("Waiting for containers to be ready ..."));
        }

        broadcast_message(
            &context_read.app_state,
            WebSocketMessage::TaskInfoUpdated(task_clone.read().await.clone()),
        )
        .await;

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
        info!("Waiting for containers to be ready: {:?}", container_ids);

        // Update task with current status
        {
            let mut task = task_clone.write().await;
            task.println(format!(
                "Waiting for {} containers to be ready ...",
                container_ids.len()
            ));
        }

        broadcast_message(
            &context_read.app_state,
            WebSocketMessage::TaskInfoUpdated(task_clone.read().await.clone()),
        )
        .await;

        // Wait for all containers to reach a non-starting state
        let container_states =
            wait_for_containers_ready(app_state, container_ids, self.timeout_seconds)
                .await
                .context("Failed to wait for containers to be ready")?;

        info!("All containers have reached a ready state");
        debug!("Container states: {:?}", container_states);

        // Update task status again
        {
            let mut task = task_clone.write().await;
            task.println(format!("All containers are ready!"));
        }

        broadcast_message(
            &context_read.app_state,
            WebSocketMessage::TaskInfoUpdated(task_clone.read().await.clone()),
        )
        .await;

        // Return the next state
        Ok(self.next_state.clone())
    }
}
