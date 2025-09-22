use tracing::info;
use uuid::Uuid;

use crate::api::websocket::WebSocketMessenger;
use crate::app_state::SharedAppState;
use crate::services::authorization::Permission;
use scotty_core::websocket::message::{LogStreamRequest, WebSocketMessage};

use super::{
    check_websocket_authentication, check_websocket_authorization, handle_websocket_auth_failure,
};

/// Handle starting a log stream via WebSocket
pub async fn handle_start_log_stream(
    state: &SharedAppState,
    client_id: Uuid,
    request: &LogStreamRequest,
) {
    info!(
        "Log stream requested by client {} for app '{}', service '{}', follow: {}, lines: {:?}",
        client_id, request.app_name, request.service_name, request.follow, request.lines
    );

    // Get user information from client
    let user = match state.messenger.get_user_for_client(client_id).await {
        Some(user) => user,
        None => {
            state
                .messenger
                .send_error(
                    client_id,
                    "Authentication required for log streaming".to_string(),
                )
                .await;
            return;
        }
    };

    // Check authorization
    let auth_result = check_websocket_authorization(
        state,
        client_id,
        &Some(user.clone()),
        &request.app_name,
        Permission::Logs,
        "log streaming",
    )
    .await;

    let authorized_user =
        match handle_websocket_auth_failure(state, client_id, auth_result, "log streaming").await {
            Some(user) => user,
            None => return,
        };

    // Look up the app
    let app = match state.apps.get_app(&request.app_name).await {
        Some(app) => app,
        None => {
            state
                .messenger
                .send_error(client_id, format!("App '{}' not found", request.app_name))
                .await;
            return;
        }
    };

    // Find the container for the specified service using the helper method
    let container = match app.find_container_by_service(&request.service_name) {
        Some(container) => container,
        None => {
            state
                .messenger
                .send_error(
                    client_id,
                    format!(
                        "Service '{}' not found in app '{}'",
                        request.service_name, request.app_name
                    ),
                )
                .await;
            return;
        }
    };

    // Check container state
    if !container.is_running() {
        state
            .messenger
            .send_error(
                client_id,
                format!(
                    "Container for service '{}' is not running (status: {:?})",
                    request.service_name, container.status
                ),
            )
            .await;
        return;
    }

    let container_id = match &container.id {
        Some(id) => id.clone(),
        None => {
            state
                .messenger
                .send_error(
                    client_id,
                    format!("Container for service '{}' has no ID", request.service_name),
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
            state
                .messenger
                .send_error(client_id, format!("Failed to start log stream: {}", e))
                .await;
        }
    }
}

/// Handle stopping a log stream via WebSocket
pub async fn handle_stop_log_stream(state: &SharedAppState, client_id: Uuid, stream_id: Uuid) {
    info!(
        "Stop log stream {} requested by client {}",
        stream_id, client_id
    );

    // Check authentication (no need for specific app permissions)
    let _user = match state.messenger.get_user_for_client(client_id).await {
        Some(user) => user,
        None => {
            state
                .messenger
                .send_error(
                    client_id,
                    "Authentication required for log stream management".to_string(),
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
            state
                .messenger
                .send_error(client_id, format!("Failed to stop log stream: {}", e))
                .await;
        }
    }
}
