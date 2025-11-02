use tracing::{debug, info};
use uuid::Uuid;

use crate::app_state::SharedAppState;
use crate::services::authorization::Permission;
use scotty_types::{ShellDataType, ShellSessionData, ShellSessionRequest};

use super::{check_websocket_authorization, handle_websocket_auth_failure};

/// Handle creating a shell session via WebSocket
pub async fn handle_create_shell_session(
    state: &SharedAppState,
    client_id: Uuid,
    request: &ShellSessionRequest,
) {
    info!(
        "Shell session creation requested by client {} for app '{}', service '{}'",
        client_id, request.app_name, request.service_name
    );

    // Get user information from client
    let user = match state.messenger.get_user_for_client(client_id).await {
        Some(user) => user,
        None => {
            state
                .messenger
                .send_error(
                    client_id,
                    "Authentication required for shell session".to_string(),
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
        Permission::Manage,
        "shell session",
    )
    .await;

    let authorized_user =
        match handle_websocket_auth_failure(state, client_id, auth_result, "shell session").await {
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

    info!(
        "Creating shell session for app '{}', service '{}' requested by user {}",
        request.app_name, request.service_name, authorized_user.email
    );

    // Create the shell session
    match state
        .shell_service
        .create_session(
            state,
            &app,
            &request.service_name,
            request.shell_command.clone(),
            client_id,
        )
        .await
    {
        Ok(session_id) => {
            info!(
                "Successfully created shell session {} for app '{}', service '{}'",
                session_id, request.app_name, request.service_name
            );
            // The ShellService will send ShellSessionCreated message
        }
        Err(e) => {
            state
                .messenger
                .send_error(client_id, format!("Failed to create shell session: {}", e))
                .await;
        }
    }
}

/// Handle shell session data (input from client)
pub async fn handle_shell_session_data(
    state: &SharedAppState,
    client_id: Uuid,
    data: &ShellSessionData,
) {
    // Only handle Input type messages - Output messages come from the server
    if !matches!(data.data_type, ShellDataType::Input) {
        debug!(
            "Ignoring non-input shell data from client {}: {:?}",
            client_id, data.data_type
        );
        return;
    }

    debug!(
        "Shell input received from client {} for session {}: {} bytes",
        client_id,
        data.session_id,
        data.data.len()
    );

    // Send input to the shell session using shared shell service
    if let Err(e) = state
        .shell_service
        .send_input(data.session_id, data.data.clone())
        .await
    {
        info!(
            "Failed to send input to shell session {}: {}",
            data.session_id, e
        );
        state
            .messenger
            .send_error(
                client_id,
                format!("Failed to send input to shell session: {}", e),
            )
            .await;
    }
}

/// Handle resizing a shell TTY via WebSocket
pub async fn handle_resize_shell_tty(
    state: &SharedAppState,
    client_id: Uuid,
    session_id: Uuid,
    width: u16,
    height: u16,
) {
    debug!(
        "Shell TTY resize requested by client {} for session {} to {}x{}",
        client_id, session_id, width, height
    );

    // Check authentication (no need for specific app permissions)
    let _user = match state.messenger.get_user_for_client(client_id).await {
        Some(user) => user,
        None => {
            state
                .messenger
                .send_error(
                    client_id,
                    "Authentication required for shell session management".to_string(),
                )
                .await;
            return;
        }
    };

    match state
        .shell_service
        .resize_tty(session_id, width, height)
        .await
    {
        Ok(()) => {
            debug!(
                "Shell TTY {} resized successfully to {}x{} by client {}",
                session_id, width, height, client_id
            );
        }
        Err(e) => {
            state
                .messenger
                .send_error(client_id, format!("Failed to resize shell TTY: {}", e))
                .await;
        }
    }
}

/// Handle terminating a shell session via WebSocket
pub async fn handle_terminate_shell_session(
    state: &SharedAppState,
    client_id: Uuid,
    session_id: Uuid,
) {
    info!(
        "Shell session termination requested by client {} for session {}",
        client_id, session_id
    );

    // Check authentication (no need for specific app permissions)
    let _user = match state.messenger.get_user_for_client(client_id).await {
        Some(user) => user,
        None => {
            state
                .messenger
                .send_error(
                    client_id,
                    "Authentication required for shell session management".to_string(),
                )
                .await;
            return;
        }
    };

    match state.shell_service.terminate_session(session_id).await {
        Ok(()) => {
            info!(
                "Shell session {} terminated successfully by client {}",
                session_id, client_id
            );
            // The ShellService will send ShellSessionEnded message
        }
        Err(e) => {
            state
                .messenger
                .send_error(
                    client_id,
                    format!("Failed to terminate shell session: {}", e),
                )
                .await;
        }
    }
}
