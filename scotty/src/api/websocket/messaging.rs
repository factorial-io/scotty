//! Centralized WebSocket messaging service
//!
//! This module provides a clean API for all WebSocket communication including:
//! - Sending messages to specific clients
//! - Broadcasting to all clients
//! - Broadcasting to task subscribers
//! - Client lifecycle management

use axum::extract::ws::Message;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::api::basic_auth::CurrentUser;
use crate::app_state::WebSocketClient;
use scotty_core::websocket::message::WebSocketMessage;

pub type WebSocketClients = Arc<Mutex<HashMap<Uuid, WebSocketClient>>>;

/// Centralized WebSocket messaging service
#[derive(Clone, Debug)]
pub struct WebSocketMessenger {
    clients: WebSocketClients,
}

impl WebSocketMessenger {
    /// Create new WebSocket messenger with given clients
    pub fn new(clients: WebSocketClients) -> Self {
        Self { clients }
    }

    /// Send a message to a specific client
    pub async fn send_to_client(
        &self,
        client_id: Uuid,
        message: WebSocketMessage,
    ) -> Result<(), WebSocketError> {
        debug!("Sending message {:?} to client {}", message, client_id);

        let clients = self.clients.lock().await;
        if let Some(client) = clients.get(&client_id) {
            let serialized =
                serde_json::to_string(&message).map_err(WebSocketError::SerializationError)?;

            client
                .sender
                .send(Message::Text(serialized.into()))
                .map_err(|e| WebSocketError::SendError(client_id, e.to_string()))?;

            crate::metrics::websocket::record_message_sent();
            Ok(())
        } else {
            Err(WebSocketError::ClientNotFound(client_id))
        }
    }

    /// Broadcast a message to all connected clients
    pub async fn broadcast_to_all(&self, message: WebSocketMessage) {
        debug!("Broadcasting message {:?} to all clients", message);

        let clients = self.clients.lock().await;
        let serialized = match serde_json::to_string(&message) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to serialize broadcast message: {}", e);
                return;
            }
        };

        let mut failed_clients = Vec::new();
        let mut sent_count = 0;
        for (&client_id, client) in clients.iter() {
            if let Err(e) = client.sender.send(Message::Text(serialized.clone().into())) {
                warn!("Failed to broadcast to client {}: {}", client_id, e);
                failed_clients.push(client_id);
            } else {
                sent_count += 1;
            }
        }

        crate::metrics::websocket::record_messages_sent(sent_count);

        if !failed_clients.is_empty() {
            debug!("Broadcast failed for {} clients", failed_clients.len());
        }
    }

    /// Send an error message to a specific client
    pub async fn send_error(&self, client_id: Uuid, error_message: String) {
        let _ = self
            .send_to_client(client_id, WebSocketMessage::Error(error_message))
            .await;
    }

    /// Send authentication success to a client
    pub async fn send_auth_success(&self, client_id: Uuid) {
        let _ = self
            .send_to_client(client_id, WebSocketMessage::AuthenticationSuccess)
            .await;
    }

    /// Send authentication failure to a client
    pub async fn send_auth_failure(&self, client_id: Uuid, reason: String) {
        let _ = self
            .send_to_client(client_id, WebSocketMessage::AuthenticationFailed { reason })
            .await;
    }

    /// Send pong response to a client
    pub async fn send_pong(&self, client_id: Uuid) {
        let _ = self.send_to_client(client_id, WebSocketMessage::Pong).await;
    }

    /// Add a new WebSocket client
    pub async fn add_client(&self, client_id: Uuid, client: WebSocketClient) {
        info!("Adding WebSocket client {}", client_id);
        let mut clients = self.clients.lock().await;
        clients.insert(client_id, client);
    }

    /// Remove a WebSocket client and clean up subscriptions
    pub async fn remove_client(&self, client_id: Uuid) {
        info!("Removing WebSocket client {}", client_id);
        let mut clients = self.clients.lock().await;
        if let Some(_client) = clients.remove(&client_id) {
            debug!("Removed WebSocket client {}", client_id);
        }
    }

    /// Authenticate a client
    pub async fn authenticate_client(
        &self,
        client_id: Uuid,
        user: CurrentUser,
    ) -> Result<(), WebSocketError> {
        let mut clients = self.clients.lock().await;
        if let Some(client) = clients.get_mut(&client_id) {
            client.authenticate(user);
            Ok(())
        } else {
            Err(WebSocketError::ClientNotFound(client_id))
        }
    }

    /// Get user for a specific client
    pub async fn get_user_for_client(&self, client_id: Uuid) -> Option<CurrentUser> {
        let clients = self.clients.lock().await;
        clients
            .get(&client_id)
            .and_then(|client| client.user.clone())
    }
}

/// WebSocket messaging errors
#[derive(Debug, thiserror::Error)]
pub enum WebSocketError {
    #[error("Client {0} not found")]
    ClientNotFound(Uuid),

    #[error("Failed to serialize message: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Failed to send message to client {0}: {1}")]
    SendError(Uuid, String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::basic_auth::CurrentUser;
    use tokio::sync::broadcast;

    async fn create_test_messenger() -> (
        WebSocketMessenger,
        Uuid,
        tokio::sync::broadcast::Receiver<axum::extract::ws::Message>,
    ) {
        let clients = Arc::new(Mutex::new(HashMap::new()));
        let messenger = WebSocketMessenger::new(clients);

        let client_id = Uuid::new_v4();
        let (tx, rx) = broadcast::channel(100);
        let client = WebSocketClient::new(tx);

        messenger.add_client(client_id, client).await;

        (messenger, client_id, rx)
    }

    #[tokio::test]
    async fn test_send_to_client() {
        let (messenger, client_id, _rx) = create_test_messenger().await;

        let result = messenger
            .send_to_client(client_id, WebSocketMessage::Pong)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_to_nonexistent_client() {
        let (messenger, _, _rx) = create_test_messenger().await;
        let fake_id = Uuid::new_v4();

        let result = messenger
            .send_to_client(fake_id, WebSocketMessage::Pong)
            .await;

        assert!(matches!(result, Err(WebSocketError::ClientNotFound(_))));
    }

    #[tokio::test]
    async fn test_client_lifecycle() {
        let clients = Arc::new(Mutex::new(HashMap::new()));
        let messenger = WebSocketMessenger::new(clients.clone());

        // Add client
        let client_id = Uuid::new_v4();
        let (tx, _rx) = broadcast::channel(100);
        let client = WebSocketClient::new(tx);

        messenger.add_client(client_id, client).await;

        // Verify client was added
        {
            let clients_guard = clients.lock().await;
            assert!(clients_guard.contains_key(&client_id));
        }

        // Remove client
        messenger.remove_client(client_id).await;

        // Verify client was removed
        {
            let clients_guard = clients.lock().await;
            assert!(!clients_guard.contains_key(&client_id));
        }
    }

    #[tokio::test]
    async fn test_authentication() {
        let (messenger, client_id, _rx) = create_test_messenger().await;

        // Authenticate
        let user = CurrentUser {
            email: "test@example.com".to_string(),
            name: "test_user".to_string(),
            picture: None,
            access_token: None,
        };

        let result = messenger.authenticate_client(client_id, user.clone()).await;
        assert!(result.is_ok());

        // Verify user was set
        let retrieved_user = messenger.get_user_for_client(client_id).await;
        assert!(retrieved_user.is_some());
        assert_eq!(retrieved_user.unwrap().email, user.email);
    }
}
