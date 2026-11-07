use tracing::{info, warn};
use uuid::Uuid;

use crate::app_state::SharedAppState;
use crate::services::AuthorizationService;
use crate::tasks::output_streaming::TaskOutputStreamError;

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

    // A task the caller may not see answers like a missing one, so its
    // existence is not revealed.
    let user_id = AuthorizationService::get_user_id_for_authorization(&user);
    let visible = match state.task_manager.get_task_details(&task_id).await {
        Some(task) => state.auth_service.can_view_task(&user_id, &task).await,
        None => false,
    };
    if !visible {
        warn!(
            "Client {} ({}) requested output of task {} it may not see",
            client_id, user_id, task_id
        );
        state
            .messenger
            .send_error(
                client_id,
                format!(
                    "Failed to start task output stream: {}",
                    TaskOutputStreamError::TaskNotFound { task_id }
                ),
            )
            .await;
        return;
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
