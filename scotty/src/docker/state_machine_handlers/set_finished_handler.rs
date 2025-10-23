use std::sync::Arc;

use scotty_core::{notification_types::Message, tasks::task_details::State};
use tokio::sync::RwLock;
use tracing::instrument;

use crate::state_machine::StateHandler;

use super::context::Context;

#[derive(Debug)]
pub struct SetFinishedHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub next_state: S,
    pub notification: Option<Message>,
}

#[async_trait::async_trait]
impl<S> StateHandler<S, Context> for SetFinishedHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    #[instrument(skip(self, _from, context))]
    async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
        // Clone data needed for messages and updates
        let (app_name, task_id, updated_task_details) = {
            let context = context.read().await;
            let app_name = context.app_data.name.clone();

            // Update task state and release write lock immediately
            let task_id = {
                let mut task_details = context.task.write().await;
                let task_id = task_details.id;
                task_details.state = State::Finished;
                task_details.output_collection_active = false;
                task_id
            };
            // Write lock released here

            // Now get updated details for broadcast (with separate read lock)
            let task_details = context.task.read().await;
            (app_name, task_id, task_details.clone())
        };

        // Now we can safely call add_task_status without holding any locks
        {
            let context_read = context.read().await;
            context_read
                .app_state
                .task_manager
                .add_task_status(
                    &task_id,
                    format!("Successfully completed operation for app '{}'", app_name),
                )
                .await;

            context_read
                .app_state
                .messenger
                .broadcast_to_all(
                    scotty_core::websocket::message::WebSocketMessage::TaskInfoUpdated(
                        updated_task_details,
                    ),
                )
                .await;
        }
        // Read lock released here before spawning
        // Send notifications in a dedicated thread.
        tokio::spawn({
            let notification = self.notification.clone();
            async move {
                let context = context.clone();
                let context = context.read().await;

                if let (Some(app_settings), Some(notification)) =
                    (&context.app_data.settings, notification)
                {
                    match crate::notification::notify::notify(
                        &context.app_state,
                        &app_settings.notify,
                        &notification,
                    )
                    .await
                    {
                        Ok(_) => {}
                        Err(err) => {
                            tracing::error!("Failed to send notification: {:?}", err);
                        }
                    }
                }
            }
        });

        Ok(self.next_state.clone())
    }
}
