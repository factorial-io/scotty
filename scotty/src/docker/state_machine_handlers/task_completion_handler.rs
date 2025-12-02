use std::sync::Arc;

use scotty_core::{notification_types::Message, tasks::task_details::State};
use tokio::sync::RwLock;
use tracing::instrument;

use crate::{docker::find_apps::inspect_app, state_machine::StateHandler};

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

        // Refresh app state to get current Docker container info
        {
            let ctx = context.read().await;
            let docker_compose_path = std::path::PathBuf::from(&ctx.app_data.docker_compose_path);

            let app_data = inspect_app(&ctx.app_state, &docker_compose_path).await?;
            ctx.app_state.apps.update_app(app_data).await?;

            // Use the shared helper - single source of truth for task completion
            let app_name = ctx.app_data.name.clone();
            let status_msg = format!("{} operation for app '{}'", status_msg_prefix, app_name);

            ctx.complete_task(target_state, status_msg, use_error_status)
                .await;
        } // Drop ctx read lock here

        // Send notifications in a dedicated thread (for both success and failure)
        if self.notification.is_some() {
            tokio::spawn({
                let notification = self.notification.clone();
                let completion_type = self.completion_type;
                let context = context.clone();
                async move {
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
