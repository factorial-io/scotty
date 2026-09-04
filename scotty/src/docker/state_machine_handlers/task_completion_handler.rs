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
        // Already terminated (e.g. the success handler failed its refresh and
        // routed here): nothing left to do, skip the second refresh.
        if context.read().await.task.read().await.state != State::Running {
            return Ok(self.next_state.clone());
        }

        // Determine state and message based on completion type
        let (mut target_state, status_msg_prefix) = match self.completion_type {
            CompletionType::Success => (State::Finished, "Successfully completed"),
            CompletionType::Failure => (State::Failed, "Operation failed for"),
        };

        // Refresh app state to get current Docker container info
        let refresh_error = {
            let ctx = context.read().await;
            let docker_compose_path = std::path::PathBuf::from(&ctx.app_data.docker_compose_path);

            // The task must reach a terminal state even if this refresh fails:
            // nothing else marks it Finished/Failed (see TaskManager), so a `?`
            // here would leave it Running forever.
            //
            // No compose file means the app was removed (destroy deletes the
            // directory before completing): nothing to refresh, not a failure.
            let refresh = if !docker_compose_path.exists() {
                tracing::debug!(
                    "Compose file {} is gone, skipping app data refresh",
                    docker_compose_path.display()
                );
                Ok(())
            } else {
                match inspect_app(&ctx.app_state, &docker_compose_path).await {
                    Ok(app_data) => ctx.app_state.apps.update_app(app_data).await.map(|_| ()),
                    Err(e) => Err(e),
                }
            };

            // Use the shared helper - single source of truth for task completion
            let app_name = ctx.app_data.name.clone();
            let mut status_msg = format!("{} operation for app '{}'", status_msg_prefix, app_name);
            if let Err(e) = &refresh {
                target_state = State::Failed;
                status_msg = format!(
                    "{} operation for app '{}', but refreshing app data failed: {}",
                    status_msg_prefix, app_name, e
                );
            }

            let is_error = matches!(target_state, State::Failed);
            ctx.complete_task(target_state, status_msg, is_error).await;
            refresh.err()
        }; // Drop ctx read lock here

        // A failed refresh fails the whole operation: propagate so the state
        // machine (and any parent state machine awaiting it) sees the error,
        // and so no success notification goes out.
        if let Some(e) = refresh_error {
            return Err(e.context("Refreshing app data after task completion failed"));
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use scotty_core::apps::app_data::AppData;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum S {
        Done,
    }

    async fn completed(app: AppData) -> (anyhow::Result<S>, scotty_types::TaskDetails) {
        let app_state = crate::api::test_utils::create_test_app_state_with_config(
            "tests/test_bearer_auth",
            None,
        )
        .await;
        let context = Context::create(app_state.clone(), &app);
        // Status lines are routed through the task manager, as in run_sm.
        let task = context.read().await.task.clone();
        let id = task.read().await.id;
        app_state.task_manager.add_task(&id, task, None).await;

        let result = TaskCompletionHandler::success(S::Done, None)
            .transition(&S::Done, context.clone())
            .await;
        let task = context.read().await.task.read().await.clone();
        (result, task)
    }

    /// A compose file outside the apps root cannot be inspected; the task
    /// must still terminate and the error must propagate.
    #[tokio::test]
    async fn failed_refresh_still_terminates_task() {
        let dir = std::env::temp_dir().join(format!("scotty-refresh-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let compose = dir.join("docker-compose.yml");
        std::fs::write(&compose, "services: {}\n").unwrap();

        let (result, task) = completed(AppData {
            name: "unrefreshable".into(),
            docker_compose_path: compose.to_string_lossy().into_owned(),
            root_directory: dir.to_string_lossy().into_owned(),
            ..Default::default()
        })
        .await;
        let _ = std::fs::remove_dir_all(&dir);

        assert!(result.is_err(), "refresh failure must propagate");
        assert_eq!(task.state, State::Failed);
        assert!(task.finish_time.is_some());
        assert!(task
            .output
            .lines
            .iter()
            .any(|l| l.content.contains("refreshing app data failed")));
    }

    /// destroy removes the app directory before completing: a missing compose
    /// file is not a refresh failure.
    #[tokio::test]
    async fn missing_compose_file_completes_successfully() {
        let (result, task) = completed(AppData {
            name: "destroyed".into(),
            docker_compose_path: "/nonexistent/scotty-test/docker-compose.yml".into(),
            root_directory: "/nonexistent/scotty-test".into(),
            ..Default::default()
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(task.state, State::Finished);
        assert!(task.finish_time.is_some());
    }
}
