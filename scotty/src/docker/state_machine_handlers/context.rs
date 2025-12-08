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

pub struct Context {
    pub app_state: SharedAppState,
    pub task: Arc<RwLock<TaskDetails>>,
    pub app_data: AppData,
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
    ///
    /// # Arguments
    /// * `target_state` - State::Finished or State::Failed
    /// * `status_message` - Message to add to task output
    /// * `is_error` - Whether to use add_task_status_error (true) or add_task_status (false)
    pub async fn complete_task(&self, target_state: State, status_message: String, is_error: bool) {
        // Get task ID first
        let task_id = self.task.read().await.id;

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

        // Now update task state and mark output collection as complete
        let updated_task_details = {
            let mut task_details = self.task.write().await;

            // Update state
            task_details.state = target_state;
            task_details.output_collection_active = false;
            task_details.finish_time = Some(chrono::Utc::now());

            // Clone for broadcast (released write lock before broadcast)
            task_details.clone()
        };

        // Broadcast task update via WebSocket
        self.app_state
            .messenger
            .broadcast_to_all(WebSocketMessage::TaskInfoUpdated(updated_task_details))
            .await;
    }
}
