use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use scotty_core::{
    apps::app_data::AppData,
    tasks::{
        running_app_context::RunningAppContext,
        task_details::{State, TaskDetails},
    },
    websocket::message::WebSocketMessage,
};
use tokio::sync::RwLock;

use crate::app_state::SharedAppState;
use crate::metrics;

pub struct Context {
    pub app_state: SharedAppState,
    pub task: Arc<RwLock<TaskDetails>>,
    pub app_data: AppData,
    /// Set by the first `complete_task` call; later callers are no-ops.
    completion_claimed: AtomicBool,
}

impl Context {
    pub async fn as_running_app_context(&self) -> RunningAppContext {
        RunningAppContext {
            task: self.task.read().await.clone(),
            app_data: self.app_data.clone(),
        }
    }

    pub fn create(app_state: SharedAppState, app_data: &AppData) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Context {
            app_state: app_state.clone(),
            app_data: app_data.clone(),
            completion_claimed: AtomicBool::new(false),
            task: Arc::new(RwLock::new(TaskDetails {
                app_name: Some(app_data.name.clone()),
                ..TaskDetails::default()
            })),
        }))
    }

    /// Complete a task with the given state (Finished or Failed)
    ///
    /// This is the single source of truth for task completion logic.
    /// It handles:
    /// - Updating task state, finish_time, and output_collection_active
    /// - Broadcasting the task update via WebSocket
    /// - Adding status messages to task output
    ///
    /// Used by both TaskCompletionHandler and helper.rs to ensure consistent behavior.
    /// A task terminates once: if it is already Finished or Failed this is a
    /// no-op and returns `false`, so a success handler that fails and then
    /// routes through the error handler does not complete the task twice.
    ///
    /// # Arguments
    /// * `target_state` - State::Finished or State::Failed
    /// * `status_message` - Message to add to task output
    /// * `is_error` - Whether to use add_task_status_error (true) or add_task_status (false)
    pub async fn complete_task(
        &self,
        target_state: State,
        status_message: String,
        is_error: bool,
    ) -> bool {
        // Claim completion atomically so concurrent callers cannot both pass;
        // the state itself is published last, together with the final status
        // line, so a poller never sees a terminal task with a truncated output.
        let task_id = {
            let task = self.task.read().await;
            if task.state != State::Running {
                return false;
            }
            task.id
        };
        // Latch only after the state check so a task terminated elsewhere can
        // never leave the claim set without a completion having happened.
        if self.completion_claimed.swap(true, Ordering::AcqRel) {
            return false;
        }

        // Add status message BEFORE marking output collection as inactive
        // This ensures the message is available for the WebSocket stream to send
        if is_error {
            self.app_state
                .task_manager
                .add_task_status_error(&task_id, status_message)
                .await;
        } else {
            self.app_state
                .task_manager
                .add_task_status(&task_id, status_message)
                .await;
        }

        // Small yield to ensure the status message write completes and is visible
        // to the WebSocket stream's next poll (which happens every 100ms)
        tokio::task::yield_now().await;

        // Now publish the terminal state and mark output collection as complete
        let updated_task_details = {
            let mut task = self.task.write().await;
            task.state = target_state;
            task.output_collection_active = false;
            let finish_time = chrono::Utc::now();
            task.finish_time = Some(finish_time);

            let duration_secs = finish_time
                .signed_duration_since(task.start_time)
                .num_milliseconds() as f64
                / 1000.0;
            metrics::metrics()
                .record_task_finished(duration_secs, matches!(task.state, State::Failed));

            // Clone for broadcast (released write lock before broadcast)
            task.clone()
        };

        // Broadcast task update via WebSocket
        self.app_state
            .messenger
            .broadcast_to_all(WebSocketMessage::TaskInfoUpdated(updated_task_details))
            .await;
        true
    }
}
