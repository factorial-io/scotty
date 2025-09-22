use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::api::auth_core::CurrentUser;
use crate::api::message::{LogStreamRequest, TaskOutputData, WebSocketMessage};
use crate::app_state::SharedAppState;
use crate::services::{authorization::Permission, AuthorizationService};

use super::ws::send_message;

/// Result of WebSocket authorization check
#[derive(Debug)]
pub enum WebSocketAuthResult {
    Authorized(CurrentUser),
    Unauthenticated,
    Unauthorized(String), // Contains the error message
}

/// Helper function to check WebSocket authorization for app-specific operations
async fn check_websocket_authorization(
    state: &SharedAppState,
    client_id: Uuid,
    user: &Option<CurrentUser>,
    app_name: &str,
    permission: Permission,
    operation: &str,
) -> WebSocketAuthResult {
    // Check authentication first
    let current_user = match user {
        Some(user) => user.clone(),
        None => {
            info!(
                "Unauthenticated WebSocket {} request denied for client {}",
                operation, client_id
            );
            return WebSocketAuthResult::Unauthenticated;
        }
    };

    // Check authorization for the specific permission
    let auth_service = &state.auth_service;
    let user_id = AuthorizationService::get_user_id_for_authorization(&current_user);

    let has_permission = auth_service
        .check_permission(&user_id, app_name, &permission)
        .await;

    if !has_permission {
        warn!(
            "Access denied: user {} lacks {} permission for app '{}' (operation: {})",
            current_user.email,
            permission.as_str(),
            app_name,
            operation
        );
        return WebSocketAuthResult::Unauthorized(format!(
            "Access denied: insufficient {} permissions for app '{}'",
            permission.as_str(),
            app_name
        ));
    }

    info!(
        "Access granted: user {} authorized for {} access to app '{}' (operation: {})",
        current_user.email,
        permission.as_str(),
        app_name,
        operation
    );

    WebSocketAuthResult::Authorized(current_user)
}

/// Helper function to handle WebSocket authorization failures
async fn handle_websocket_auth_failure(
    state: &SharedAppState,
    client_id: Uuid,
    auth_result: WebSocketAuthResult,
    operation: &str,
) -> Option<CurrentUser> {
    match auth_result {
        WebSocketAuthResult::Authorized(user) => Some(user),
        WebSocketAuthResult::Unauthenticated => {
            send_message(
                state,
                client_id,
                WebSocketMessage::Error(format!("Authentication required for {}", operation)),
            )
            .await;
            None
        }
        WebSocketAuthResult::Unauthorized(error_msg) => {
            send_message(state, client_id, WebSocketMessage::Error(error_msg)).await;
            None
        }
    }
}

/// Simple authentication check for operations that don't require specific app permissions
async fn check_websocket_authentication(
    state: &SharedAppState,
    client_id: Uuid,
    user: &Option<CurrentUser>,
    operation: &str,
) -> Option<CurrentUser> {
    match user {
        Some(user) => Some(user.clone()),
        None => {
            info!(
                "Unauthenticated WebSocket {} request denied for client {}",
                operation, client_id
            );
            send_message(
                state,
                client_id,
                WebSocketMessage::Error(format!("Authentication required for {}", operation)),
            )
            .await;
            None
        }
    }
}

#[instrument(skip(state))]
pub async fn handle_websocket_message(
    state: &SharedAppState,
    client_id: Uuid,
    msg: &WebSocketMessage,
) {
    match msg {
        WebSocketMessage::Ping => {
            send_message(state, client_id, WebSocketMessage::Pong).await;
        }
        WebSocketMessage::Pong => {}
        WebSocketMessage::Error(_) => {}

        WebSocketMessage::Authenticate { token } => {
            handle_authentication(state, client_id, token).await;
        }

        WebSocketMessage::StartLogStream(request) => {
            handle_start_log_stream(state, client_id, request).await;
        }

        WebSocketMessage::StopLogStream { stream_id } => {
            handle_stop_log_stream(state, client_id, *stream_id).await;
        }

        WebSocketMessage::StartTaskOutputStream {
            task_id,
            from_beginning,
        } => {
            handle_start_task_output_stream(state, client_id, *task_id, *from_beginning).await;
        }

        WebSocketMessage::StopTaskOutputStream { task_id } => {
            handle_stop_task_output_stream(state, client_id, *task_id).await;
        }

        _ => {
            warn!("Unhandled WebSocket message type: {:?}", msg);
        }
    }
}

/// Handle WebSocket authentication
async fn handle_authentication(state: &SharedAppState, client_id: Uuid, token: &str) {
    info!("Authentication attempt for WebSocket client {}", client_id);

    // Extract user from token using the centralized authentication logic
    let user = match crate::api::auth_core::authenticate_user_from_token(state, token).await {
        Ok(user) => user,
        Err(e) => {
            warn!("Authentication failed for client {}: {}", client_id, e);
            send_message(
                state,
                client_id,
                WebSocketMessage::AuthenticationFailed {
                    reason: "Invalid or expired token".to_string(),
                },
            )
            .await;
            return;
        }
    };

    // Update client with authenticated user
    {
        let mut clients = state.clients.lock().await;
        if let Some(client) = clients.get_mut(&client_id) {
            client.authenticate(user.clone());
            info!(
                "WebSocket client {} successfully authenticated as {}",
                client_id, user.email
            );
        } else {
            warn!("Client {} not found for authentication", client_id);
            return;
        }
    }

    // Send success message
    send_message(state, client_id, WebSocketMessage::AuthenticationSuccess).await;
}

/// Handle starting a log stream via WebSocket
async fn handle_start_log_stream(
    state: &SharedAppState,
    client_id: Uuid,
    request: &LogStreamRequest,
) {
    info!(
        "Log stream requested by client {} for app '{}', service '{}', follow: {}, lines: {:?}",
        client_id, request.app_name, request.service_name, request.follow, request.lines
    );

    // Get user from clients map
    let user = {
        let clients = state.clients.lock().await;
        if let Some(client) = clients.get(&client_id) {
            client.user.clone()
        } else {
            send_message(
                state,
                client_id,
                WebSocketMessage::Error("Client not found".to_string()),
            )
            .await;
            return;
        }
    };

    // Check authorization
    let auth_result = check_websocket_authorization(
        state,
        client_id,
        &user,
        &request.app_name,
        Permission::Logs,
        "log streaming",
    )
    .await;

    let authorized_user = match handle_websocket_auth_failure(
        state,
        client_id,
        auth_result,
        "log streaming",
    )
    .await
    {
        Some(user) => user,
        None => return,
    };

    // Look up the app
    let app = match state.apps.get_app(&request.app_name).await {
        Some(app) => app,
        None => {
            send_message(
                state,
                client_id,
                WebSocketMessage::Error(format!("App '{}' not found", request.app_name)),
            )
            .await;
            return;
        }
    };

    // Find the container for the specified service using the helper method
    let container = match app.find_container_by_service(&request.service_name) {
        Some(container) => container,
        None => {
            send_message(
                state,
                client_id,
                WebSocketMessage::Error(format!(
                    "Service '{}' not found in app '{}'",
                    request.service_name, request.app_name
                )),
            )
            .await;
            return;
        }
    };

    // Check container state
    if !container.is_running() {
        send_message(
            state,
            client_id,
            WebSocketMessage::Error(format!(
                "Container for service '{}' is not running (status: {:?})",
                request.service_name, container.status
            )),
        )
        .await;
        return;
    }

    let container_id = match &container.id {
        Some(id) => id.clone(),
        None => {
            send_message(
                state,
                client_id,
                WebSocketMessage::Error(format!(
                    "Container for service '{}' has no ID",
                    request.service_name
                )),
            )
            .await;
            return;
        }
    };

    info!(
        "Starting log stream for container '{}' (app: '{}', service: '{}') requested by user {}",
        container_id, request.app_name, request.service_name, authorized_user.email
    );

    // Start the log streaming
    let tail = request.lines.map(|n| n.to_string());
    match state
        .logs_service
        .start_stream(
            state,
            &app,
            &request.service_name,
            request.follow,
            tail,
            Some(client_id),
        )
        .await
    {
        Ok(stream_id) => {
            info!(
                "Successfully started log stream {} for container '{}'",
                stream_id, container_id
            );
            // The LogStreamingService will send LogsStreamStarted message
        }
        Err(e) => {
            send_message(
                state,
                client_id,
                WebSocketMessage::Error(format!("Failed to start log stream: {}", e)),
            )
            .await;
        }
    }
}

/// Handle stopping a log stream via WebSocket
async fn handle_stop_log_stream(state: &SharedAppState, client_id: Uuid, stream_id: Uuid) {
    info!(
        "Stop log stream {} requested by client {}",
        stream_id, client_id
    );

    // Check authentication (no need for specific app permissions)
    let user =
        {
            let clients = state.clients.lock().await;
            if let Some(client) = clients.get(&client_id) {
                client.user.clone()
            } else {
                send_message(
                    state,
                    client_id,
                    WebSocketMessage::Error("Client not found".to_string()),
                )
                .await;
                return;
            }
        };

    match check_websocket_authentication(state, client_id, &user, "log stream management").await {
        Some(_) => {
            // User is authenticated
        }
        None => {
            send_message(
                state,
                client_id,
                WebSocketMessage::Error(
                    "Authentication required for log stream management".to_string(),
                ),
            )
            .await;
            return;
        }
    };

    match state.logs_service.stop_stream(stream_id).await {
        Ok(()) => {
            info!(
                "Log stream {} stopped successfully by client {}",
                stream_id, client_id
            );
            // Stream stopped successfully
            // The LogStreamingService will send LogsStreamEnded message
        }
        Err(e) => {
            send_message(
                state,
                client_id,
                WebSocketMessage::Error(format!("Failed to stop log stream: {}", e)),
            )
            .await;
        }
    }
}

/// Handle starting a task output stream via WebSocket
async fn handle_start_task_output_stream(
    state: &SharedAppState,
    client_id: Uuid,
    task_id: Uuid,
    from_beginning: bool,
) {
    info!(
        "Task output stream requested by client {} for task {}, from_beginning: {}",
        client_id, task_id, from_beginning
    );

    // Check authentication
    let user = {
        let clients = state.clients.lock().await;
        if let Some(client) = clients.get(&client_id) {
            client.user.clone()
        } else {
            send_message(
                state,
                client_id,
                WebSocketMessage::Error("Client not found".to_string()),
            )
            .await;
            return;
        }
    };

    // Verify user is authenticated
    if user.is_none() {
        send_message(
            state,
            client_id,
            WebSocketMessage::Error("Authentication required for task output streaming".to_string()),
        )
        .await;
        return;
    }

    // Check if task exists and get its output
    let task_output = match state.task_manager.get_task_output(&task_id).await {
        Some(output) => output,
        None => {
            send_message(
                state,
                client_id,
                WebSocketMessage::Error(format!("Task {} not found", task_id)),
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
                &user,
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
    {
        let mut clients = state.clients.lock().await;
        if let Some(client) = clients.get_mut(&client_id) {
            client.subscribe_to_task(task_id);
            info!(
                "Client {} subscribed to task {} output",
                client_id, task_id
            );
        }
    }

    // Send stream started notification
    send_message(
        state,
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
            send_message(
                state,
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
                send_message(
                    state,
                    client_id,
                    WebSocketMessage::TaskOutputStreamEnded {
                        task_id,
                        reason: "completed".to_string(),
                    },
                )
                .await;
            }
            State::Failed => {
                send_message(
                    state,
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
async fn handle_stop_task_output_stream(state: &SharedAppState, client_id: Uuid, task_id: Uuid) {
    info!(
        "Task output stream stop requested by client {} for task {}",
        client_id, task_id
    );

    // Unsubscribe the client from this task's output
    {
        let mut clients = state.clients.lock().await;
        if let Some(client) = clients.get_mut(&client_id) {
            client.unsubscribe_from_task(task_id);
            info!(
                "Client {} unsubscribed from task {} output",
                client_id, task_id
            );
        }
    }

    // Send confirmation
    send_message(
        state,
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
    let mut clients = state.clients.lock().await;
    for (client_id, client) in clients.iter_mut() {
        if client.is_subscribed_to_task(task_id) {
            client.unsubscribe_from_task(*task_id);
            info!(
                "Cleaned up task {} subscription for client {}",
                task_id, client_id
            );

            // Notify the client that the stream has ended
            send_message(
                state,
                *client_id,
                WebSocketMessage::TaskOutputStreamEnded {
                    task_id: *task_id,
                    reason: "task expired".to_string(),
                },
            )
            .await;
        }
    }
}