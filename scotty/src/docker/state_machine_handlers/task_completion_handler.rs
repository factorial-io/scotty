use std::sync::Arc;

use scotty_core::{notification_types::Message, tasks::task_details::State};
use tokio::sync::RwLock;
use tracing::instrument;

use crate::state_machine::StateHandler;

use super::context::Context;

/// Represents the completion type of a task
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionType {
    Success,
    Failure,
}

/// Unified handler for both successful and failed task completions
///
/// This handler consolidates the logic for finishing tasks, whether they
/// succeed or fail. It handles:
/// - Setting task state (Finished/Failed)
/// - Marking output collection as inactive
/// - Setting finish time
/// - Broadcasting status updates
/// - Sending optional notifications
#[derive(Debug)]
pub struct TaskCompletionHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub next_state: S,
    pub completion_type: CompletionType,
    pub notification: Option<Message>,
}

impl<S> TaskCompletionHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    /// Create a handler for successful task completion
    ///
    /// # Arguments
    /// * `next_state` - The state to transition to after completion
    /// * `notification` - Optional notification to send (e.g., "App created successfully")
    pub fn success(next_state: S, notification: Option<Message>) -> Self {
        Self {
            next_state,
            completion_type: CompletionType::Success,
            notification,
        }
    }

    /// Create a handler for failed task completion
    ///
    /// # Arguments
    /// * `next_state` - The state to transition to after completion
    /// * `notification` - Optional notification to send (e.g., "App creation failed")
    pub fn failure(next_state: S, notification: Option<Message>) -> Self {
        Self {
            next_state,
            completion_type: CompletionType::Failure,
            notification,
        }
    }
}

#[async_trait::async_trait]
impl<S> StateHandler<S, Context> for TaskCompletionHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    #[instrument(skip(self, _from, context))]
    async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
        // Determine state and message based on completion type
        let (target_state, status_msg_prefix, use_error_status) = match self.completion_type {
            CompletionType::Success => (State::Finished, "Successfully completed", false),
            CompletionType::Failure => (State::Failed, "Operation failed for", true),
        };

        // Clone data needed for messages and updates
        let (app_name, task_id, updated_task_details) = {
            let context = context.read().await;
            let app_name = context.app_data.name.clone();

            // Update task state and release write lock immediately
            let task_id = {
                let mut task_details = context.task.write().await;
                let task_id = task_details.id;
                task_details.state = target_state;
                task_details.output_collection_active = false;
                task_details.finish_time = Some(chrono::Utc::now());
                task_id
            };
            // Write lock released here

            // Now get updated details for broadcast (with separate read lock)
            let task_details = context.task.read().await;
            (app_name, task_id, task_details.clone())
        };

        // Add status message without holding any locks
        {
            let context_read = context.read().await;
            let status_msg = format!("{} operation for app '{}'", status_msg_prefix, app_name);

            if use_error_status {
                context_read
                    .app_state
                    .task_manager
                    .add_task_status_error(&task_id, status_msg)
                    .await;
            } else {
                context_read
                    .app_state
                    .task_manager
                    .add_task_status(&task_id, status_msg)
                    .await;
            }

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

        // Send notifications in a dedicated thread (for both success and failure)
        if self.notification.is_some() {
            tokio::spawn({
                let notification = self.notification.clone();
                let completion_type = self.completion_type;
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
                            Ok(_) => {
                                tracing::debug!(
                                    "Sent {:?} notification for app '{}'",
                                    completion_type,
                                    context.app_data.name
                                );
                            }
                            Err(err) => {
                                tracing::error!(
                                    "Failed to send {:?} notification for app '{}': {:?}",
                                    completion_type,
                                    context.app_data.name,
                                    err
                                );
                            }
                        }
                    }
                }
            });
        }

        Ok(self.next_state.clone())
    }
}
