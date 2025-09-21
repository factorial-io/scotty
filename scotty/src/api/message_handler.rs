use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::api::auth_core::CurrentUser;
use crate::api::message::{LogStreamRequest, WebSocketMessage};
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
    client_id: Uuid,
    user: &Option<CurrentUser>,
    operation: &str,
) -> Option<CurrentUser> {
    match user {
        Some(user) => {
            info!(
                "WebSocket {} authorized for user {} (client {})",
                operation, user.email, client_id
            );
            Some(user.clone())
        }
        None => {
            info!(
                "Unauthenticated WebSocket {} request denied for client {}",
                operation, client_id
            );
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

    // Check authorization using helper function
    let auth_result = check_websocket_authorization(
        state,
        client_id,
        &user,
        &request.app_name,
        Permission::Logs,
        "log streaming",
    )
    .await;

    let _current_user =
        match handle_websocket_auth_failure(state, client_id, auth_result, "log streaming").await {
            Some(user) => user,
            None => return, // Authorization failed, error message already sent
        };

    // Get app data to validate app and service exist
    let app_data = match state.apps.get_app(&request.app_name).await {
        Some(data) => data,
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

    // Validate service exists
    if app_data
        .find_container_by_service(&request.service_name)
        .is_none()
    {
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

    // TODO: Add authorization check here when WebSocket auth is implemented
    // For now, we'll proceed without auth (this should be secured later)

    // Use the shared log streaming service

    // Convert tail parameter from lines count
    let tail = request.lines.map(|lines| lines.to_string());

    match state
        .logs_service
        .start_stream(
            state,
            &app_data,
            &request.service_name,
            request.follow,
            tail,
            Some(client_id), // Associate with this WebSocket client
        )
        .await
    {
        Ok(stream_id) => {
            info!(
                "Log stream {} started successfully for client {} (app: '{}', service: '{}')",
                stream_id, client_id, request.app_name, request.service_name
            );
            // Stream started successfully, LogStreamingService will send updates via WebSocket
            // No need to send additional response here
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
        "Log stream stop requested by client {} for stream {}",
        client_id, stream_id
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

    // Check authentication using helper function
    let _current_user =
        match check_websocket_authentication(client_id, &user, "log stream stop").await {
            Some(user) => user,
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
