use tracing::{debug, info};
use uuid::Uuid;

use crate::app_state::SharedAppState;
use crate::docker::services::shell::ShellService;
use scotty_types::{ShellDataType, ShellSessionData};

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

    // Create shell service
    let shell_service = ShellService::new(state.docker.clone(), state.settings.shell.clone());

    // Send input to the shell session
    if let Err(e) = shell_service
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
