use tracing::info;
use uuid::Uuid;

use crate::app_state::SharedAppState;
use crate::services::authorization::Permission;
use scotty_core::websocket::message::WebSocketMessage;
use scotty_types::TaskOutputData;

use super::{check_websocket_authorization, handle_websocket_auth_failure, WebSocketAuthResult};

/// Handle starting a task output stream via WebSocket
pub async fn handle_start_task_output_stream(
    state: &SharedAppState,
    client_id: Uuid,
    task_id: Uuid,
    from_beginning: bool,
) {
    info!(
        "Task output stream requested by client {} for task {}, from_beginning: {}",
        client_id, task_id, from_beginning
    );

    // Get user information from client
    let user = match state.messenger.get_user_for_client(client_id).await {
        Some(user) => user,
        None => {
            state
                .messenger
                .send_error(
                    client_id,
                    "Authentication required for task output streaming".to_string(),
                )
                .await;
            return;
        }
    };

    // Check if task exists and get its output
    let task_output = match state.task_manager.get_task_output(&task_id).await {
        Some(output) => output,
        None => {
            state
                .messenger
                .send_error(client_id, format!("Task {} not found", task_id))
                .await;
            return;
        }
    };

    // Check if task has an app_name for authorization
    let task_details = state.task_manager.get_task_details(&task_id).await;
    if let Some(details) = task_details {
        if let Some(app_name) = &details.app_name {
            // Check authorization for the app
            let auth_result = check_websocket_authorization(
                state,
                client_id,
                &Some(user),
                app_name,
                Permission::View,
                "task output stream",
            )
            .await;

            if !matches!(auth_result, WebSocketAuthResult::Authorized(_)) {
                handle_websocket_auth_failure(state, client_id, auth_result, "task output stream")
                    .await;
                return;
            }
        }
    }

    // Subscribe the client to this task's output
    // Subscribe client to task updates
    let _ = state.messenger.subscribe_to_task(client_id, task_id).await;

    // Send stream started notification
    let _ = state
        .messenger
        .send_to_client(
            client_id,
            WebSocketMessage::TaskOutputStreamStarted {
                task_id,
                total_lines: task_output.total_lines_processed,
            },
        )
        .await;

    // Send existing output if requested
    if from_beginning && !task_output.lines.is_empty() {
        // Send all existing lines in batches of 1000 to avoid overwhelming the WebSocket
        const BATCH_SIZE: usize = 1000;
        let lines: Vec<_> = task_output.lines.into_iter().collect();
        let chunks = lines.chunks(BATCH_SIZE);
        let total_chunks = chunks.len();

        for (i, chunk) in chunks.enumerate() {
            let is_last_batch = i == total_chunks - 1;
            let _ = state
                .messenger
                .send_to_client(
                    client_id,
                    WebSocketMessage::TaskOutputData(TaskOutputData {
                        task_id,
                        lines: chunk.to_vec(),
                        is_historical: true,
                        has_more: !is_last_batch,
                    }),
                )
                .await;
        }
    }

    // Check if task is already finished and send end message if so
    if let Some(details) = state.task_manager.get_task_details(&task_id).await {
        use scotty_core::tasks::task_details::State;
        match details.state {
            State::Finished => {
                let _ = state
                    .messenger
                    .send_to_client(
                        client_id,
                        WebSocketMessage::TaskOutputStreamEnded {
                            task_id,
                            reason: "completed".to_string(),
                        },
                    )
                    .await;
            }
            State::Failed => {
                let _ = state
                    .messenger
                    .send_to_client(
                        client_id,
                        WebSocketMessage::TaskOutputStreamEnded {
                            task_id,
                            reason: "failed".to_string(),
                        },
                    )
                    .await;
            }
            State::Running => {
                // Task is still running, client will receive live updates
            }
        }
    }
}

/// Handle stopping a task output stream via WebSocket
pub async fn handle_stop_task_output_stream(
    state: &SharedAppState,
    client_id: Uuid,
    task_id: Uuid,
) {
    info!(
        "Task output stream stop requested by client {} for task {}",
        client_id, task_id
    );

    // Unsubscribe the client from this task's output
    let _ = state
        .messenger
        .unsubscribe_from_task(client_id, task_id)
        .await;
    info!(
        "Client {} unsubscribed from task {} output",
        client_id, task_id
    );

    // Send confirmation
    let _ = state
        .messenger
        .send_to_client(
            client_id,
            WebSocketMessage::TaskOutputStreamEnded {
                task_id,
                reason: "stopped by client".to_string(),
            },
        )
        .await;
}

/// Clean up task subscriptions for all clients when a task is removed
pub async fn cleanup_task_subscriptions(state: &SharedAppState, task_id: &Uuid) {
    // This is now handled by the messenger's cleanup_task_subscriptions method
    state.messenger.cleanup_task_subscriptions(*task_id).await;
}
