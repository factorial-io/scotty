pub mod auth;
pub mod logs;
pub mod shell;
pub mod tasks;

#[cfg(test)]
mod session_ownership_tests;

use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::api::auth_core::CurrentUser;
use crate::app_state::SharedAppState;
use crate::services::{
    authorization::{
        decision::{Decision, Target},
        Permission,
    },
    AuthorizationService,
};
use scotty_core::websocket::message::WebSocketMessage;

/// Result of WebSocket authorization check
#[derive(Debug)]
pub enum WebSocketAuthResult {
    Authorized(CurrentUser),
    Unauthenticated,
    Unauthorized(String), // Contains the error message
}

/// Helper function to check WebSocket authorization for app-specific operations
pub async fn check_websocket_authorization(
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

    if let Decision::Deny(reason) =
        auth_service.decide(&user_id, permission.into(), Target::App(app_name))
    {
        warn!(
            "Access denied: user {} for app '{}' (operation: {}): {}",
            current_user.email, app_name, operation, reason
        );
        return WebSocketAuthResult::Unauthorized(format!(
            "Access denied: {reason} for app '{app_name}'"
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
pub async fn handle_websocket_auth_failure(
    state: &SharedAppState,
    client_id: Uuid,
    auth_result: WebSocketAuthResult,
    operation: &str,
) -> Option<CurrentUser> {
    match auth_result {
        WebSocketAuthResult::Authorized(user) => Some(user),
        WebSocketAuthResult::Unauthenticated => {
            state
                .messenger
                .send_error(
                    client_id,
                    format!("Authentication required for {}", operation),
                )
                .await;
            None
        }
        WebSocketAuthResult::Unauthorized(error_msg) => {
            state.messenger.send_error(client_id, error_msg).await;
            None
        }
    }
}

/// Main WebSocket message dispatcher
#[instrument(skip(state, msg))]
pub async fn handle_websocket_message(
    state: &SharedAppState,
    client_id: Uuid,
    msg: &WebSocketMessage,
) {
    debug!("Received WebSocket message: {msg}"); // Uses Display impl which masks token
    match msg {
        WebSocketMessage::Ping => {
            state.messenger.send_pong(client_id).await;
        }
        WebSocketMessage::Pong => {}
        WebSocketMessage::Error(_) => {}

        WebSocketMessage::Authenticate { token } => {
            auth::handle_authentication(state, client_id, token).await;
        }

        WebSocketMessage::StartLogStream(request) => {
            logs::handle_start_log_stream(state, client_id, request).await;
        }

        WebSocketMessage::StopLogStream { stream_id } => {
            logs::handle_stop_log_stream(state, client_id, *stream_id).await;
        }

        WebSocketMessage::StartTaskOutputStream {
            task_id,
            from_beginning,
        } => {
            tasks::handle_start_task_output_stream(state, client_id, *task_id, *from_beginning)
                .await;
        }

        WebSocketMessage::StopTaskOutputStream { task_id } => {
            tasks::handle_stop_task_output_stream(state, client_id, *task_id).await;
        }

        WebSocketMessage::CreateShellSession(request) => {
            shell::handle_create_shell_session(state, client_id, request).await;
        }

        WebSocketMessage::ShellSessionData(data) => {
            shell::handle_shell_session_data(state, client_id, data).await;
        }

        WebSocketMessage::ResizeShellTty {
            session_id,
            width,
            height,
        } => {
            shell::handle_resize_shell_tty(state, client_id, *session_id, *width, *height).await;
        }

        WebSocketMessage::TerminateShellSession { session_id } => {
            shell::handle_terminate_shell_session(state, client_id, *session_id).await;
        }

        WebSocketMessage::CloseShellStdin { session_id } => {
            shell::handle_close_shell_stdin(state, client_id, *session_id).await;
        }

        _ => {
            warn!("Unhandled WebSocket message type: {:?}", msg);
        }
    }
}
