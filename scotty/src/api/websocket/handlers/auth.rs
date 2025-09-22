use tracing::{info, warn};
use uuid::Uuid;

use crate::api::websocket::client::send_message;
use crate::api::websocket::message::WebSocketMessage;
use crate::app_state::SharedAppState;

/// Handle WebSocket authentication
pub async fn handle_authentication(state: &SharedAppState, client_id: Uuid, token: &str) {
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
