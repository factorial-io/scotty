use tracing::{info, warn};
use uuid::Uuid;

use crate::app_state::SharedAppState;

/// Handle WebSocket authentication
pub async fn handle_authentication(state: &SharedAppState, client_id: Uuid, token: &str) {
    info!("Authentication attempt for WebSocket client {}", client_id);

    // Extract user from token using the centralized authentication logic
    let user = match crate::api::auth_core::authenticate_user_from_token(state, token).await {
        Ok(user) => user,
        Err(e) => {
            warn!("Authentication failed for client {}: {}", client_id, e);
            state
                .messenger
                .send_auth_failure(client_id, "Invalid or expired token".to_string())
                .await;
            return;
        }
    };

    // Update client with authenticated user using messenger
    if let Err(e) = state
        .messenger
        .authenticate_client(client_id, user.clone())
        .await
    {
        warn!("Failed to authenticate client {}: {}", client_id, e);
        return;
    }

    info!(
        "WebSocket client {} successfully authenticated as {}",
        client_id, user.email
    );

    // Send success message
    state.messenger.send_auth_success(client_id).await;
}
