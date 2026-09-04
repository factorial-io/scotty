use std::sync::Arc;

use anyhow::Context as _;
use scotty_core::notification_types::Message;
use tokio::sync::RwLock;
use tracing::instrument;

use crate::{
    docker::find_apps::inspect_app,
    state_machine::StateHandler,
    tasks::actor::{FailureKind, Outcome},
};

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
        let ctx = context.read().await;
        // Already terminated (e.g. the success handler failed its refresh and
        // routed here): nothing left to do.
        if ctx.task.is_terminated() {
            return Ok(self.next_state.clone());
        }
        let app_name = ctx.app_data.name.clone();

        // Refresh app state to get current Docker container info. No compose
        // file means the app was removed (destroy deletes the directory before
        // completing): nothing to refresh, not a failure.
        let docker_compose_path = std::path::PathBuf::from(&ctx.app_data.docker_compose_path);
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

        let outcome = match (&refresh, self.completion_type) {
            (Err(e), _) => Outcome::failed(
                FailureKind::RefreshFailed,
                format!(
                    "Operation for app '{app_name}' ended, but refreshing app data failed: {e:#}"
                ),
            ),
            (Ok(()), CompletionType::Success) => Outcome::finished(format!(
                "Successfully completed operation for app '{app_name}'"
            )),
            (Ok(()), CompletionType::Failure) => Outcome::failed(
                FailureKind::StepFailed,
                format!("Operation failed for app '{app_name}'"),
            ),
        };
        ctx.task.terminate(outcome).await;
        drop(ctx);

        // A failed refresh fails the whole operation: propagate so the state
        // machine (and any parent awaiting it) sees the error and no success
        // notification goes out.
        refresh.context("Refreshing app data after task completion failed")?;

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
    use scotty_core::tasks::task_details::State;

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
        let context = Context::create(app_state.clone(), &app).await;
        let result = TaskCompletionHandler::success(S::Done, None)
            .transition(&S::Done, context.clone())
            .await;

        let id = context.read().await.task.id();
        let mut rx = app_state.task_manager.subscribe(&id).await.unwrap();
        let task = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let task = rx.borrow_and_update().clone();
                if task.state != State::Running {
                    return (*task).clone();
                }
                rx.changed().await.unwrap();
            }
        })
        .await
        .expect("task never terminated");
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
        assert!(task
            .output
            .lines
            .iter()
            .any(|l| l.content == "Successfully completed operation for app 'destroyed'"));
    }
}
