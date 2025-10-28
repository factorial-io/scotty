use tracing::info;
use uuid::Uuid;

use crate::app_state::SharedAppState;
use crate::services::authorization::Permission;

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

    // Start the self-contained streaming service
    if let Err(e) = state
        .task_output_service
        .start_task_output_stream(state, task_id, client_id, from_beginning)
        .await
    {
        state
            .messenger
            .send_error(
                client_id,
                format!("Failed to start task output stream: {}", e),
            )
            .await;
    }
}

/// Handle stopping a task output stream via WebSocket
pub async fn handle_stop_task_output_stream(
    _state: &SharedAppState,
    client_id: Uuid,
    task_id: Uuid,
) {
    info!(
        "Task output stream stop requested by client {} for task {} (streams are self-terminating, no action needed)",
        client_id, task_id
    );

    // With the new self-contained streaming approach, streams terminate naturally
    // when the client disconnects or the task completes. No explicit stop needed.
}
