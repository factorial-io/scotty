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
            let serialized = serde_json::to_string(&message)
                .map_err(|e| WebSocketError::SerializationError(e))?;

            client
                .sender
                .send(Message::Text(serialized.into()))
                .map_err(|e| WebSocketError::SendError(client_id, e.to_string()))?;

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
        for (&client_id, client) in clients.iter() {
            if let Err(e) = client.sender.send(Message::Text(serialized.clone().into())) {
                warn!("Failed to broadcast to client {}: {}", client_id, e);
                failed_clients.push(client_id);
            }
        }

        if !failed_clients.is_empty() {
            debug!("Broadcast failed for {} clients", failed_clients.len());
        }
    }

    /// Broadcast a message to clients subscribed to a specific task
    pub async fn broadcast_to_task_subscribers(&self, task_id: Uuid, message: WebSocketMessage) {
        debug!(
            "Broadcasting message {:?} to task {} subscribers",
            message, task_id
        );

        let clients = self.clients.lock().await;
        let serialized = match serde_json::to_string(&message) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to serialize task broadcast message: {}", e);
                return;
            }
        };

        let mut subscriber_count = 0;
        let mut failed_clients = Vec::new();

        for (&client_id, client) in clients.iter() {
            if client.is_subscribed_to_task(&task_id) {
                subscriber_count += 1;
                if let Err(e) = client.sender.send(Message::Text(serialized.clone().into())) {
                    warn!("Failed to send to task subscriber {}: {}", client_id, e);
                    failed_clients.push(client_id);
                }
            }
        }

        debug!(
            "Sent to {} task subscribers, {} failed",
            subscriber_count,
            failed_clients.len()
        );
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
        if let Some(client) = clients.remove(&client_id) {
            debug!(
                "Removed client {} with {} task subscriptions",
                client_id,
                client.task_output_subscriptions.len()
            );
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

    /// Subscribe a client to task output
    pub async fn subscribe_to_task(
        &self,
        client_id: Uuid,
        task_id: Uuid,
    ) -> Result<(), WebSocketError> {
        let mut clients = self.clients.lock().await;
        if let Some(client) = clients.get_mut(&client_id) {
            client.subscribe_to_task(task_id);
            debug!("Client {} subscribed to task {}", client_id, task_id);
            Ok(())
        } else {
            Err(WebSocketError::ClientNotFound(client_id))
        }
    }

    /// Unsubscribe a client from task output
    pub async fn unsubscribe_from_task(
        &self,
        client_id: Uuid,
        task_id: Uuid,
    ) -> Result<(), WebSocketError> {
        let mut clients = self.clients.lock().await;
        if let Some(client) = clients.get_mut(&client_id) {
            client.unsubscribe_from_task(task_id);
            debug!("Client {} unsubscribed from task {}", client_id, task_id);
            Ok(())
        } else {
            Err(WebSocketError::ClientNotFound(client_id))
        }
    }

    /// Clean up all subscriptions for a specific task across all clients
    pub async fn cleanup_task_subscriptions(&self, task_id: Uuid) {
        let mut clients = self.clients.lock().await;
        let mut cleaned_count = 0;

        for (&_client_id, client) in clients.iter_mut() {
            if client.is_subscribed_to_task(&task_id) {
                client.unsubscribe_from_task(task_id);
                cleaned_count += 1;

                // Send stream ended notification
                let message = WebSocketMessage::TaskOutputStreamEnded {
                    task_id,
                    reason: "task_cleanup".to_string(),
                };

                if let Ok(serialized) = serde_json::to_string(&message) {
                    let _ = client.sender.send(Message::Text(serialized.into()));
                }
            }
        }

        if cleaned_count > 0 {
            info!(
                "Cleaned up task {} subscriptions for {} clients",
                task_id, cleaned_count
            );
        }
    }

    /// Get current client count
    pub async fn client_count(&self) -> usize {
        let clients = self.clients.lock().await;
        clients.len()
    }

    /// Get authenticated client count
    pub async fn authenticated_client_count(&self) -> usize {
        let clients = self.clients.lock().await;
        clients.values().filter(|c| c.is_authenticated()).count()
    }

    /// Get user for a specific client
    pub async fn get_user_for_client(&self, client_id: Uuid) -> Option<CurrentUser> {
        let clients = self.clients.lock().await;
        clients
            .get(&client_id)
            .and_then(|client| client.user.clone())
    }

    /// Get client info for a specific client (for debugging)
    pub async fn get_client_info(&self, client_id: Uuid) -> Option<ClientInfo> {
        let clients = self.clients.lock().await;
        clients.get(&client_id).map(|client| ClientInfo {
            id: client_id,
            is_authenticated: client.is_authenticated(),
            task_subscriptions: client.task_output_subscriptions.len(),
            username: client.user.as_ref().map(|u| u.name.clone()),
        })
    }

    /// Get info for all clients (for debugging/monitoring)
    pub async fn get_all_client_info(&self) -> Vec<ClientInfo> {
        let clients = self.clients.lock().await;
        clients
            .iter()
            .map(|(&id, client)| ClientInfo {
                id,
                is_authenticated: client.is_authenticated(),
                task_subscriptions: client.task_output_subscriptions.len(),
                username: client.user.as_ref().map(|u| u.name.clone()),
            })
            .collect()
    }
}

/// Information about a WebSocket client for debugging/monitoring
#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub id: Uuid,
    pub is_authenticated: bool,
    pub task_subscriptions: usize,
    pub username: Option<String>,
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
        let messenger = WebSocketMessenger::new(clients);

        // Start with no clients
        assert_eq!(messenger.client_count().await, 0);

        // Add client
        let client_id = Uuid::new_v4();
        let (tx, _rx) = broadcast::channel(100);
        let client = WebSocketClient::new(tx);

        messenger.add_client(client_id, client).await;
        assert_eq!(messenger.client_count().await, 1);

        // Remove client
        messenger.remove_client(client_id).await;
        assert_eq!(messenger.client_count().await, 0);
    }

    #[tokio::test]
    async fn test_task_subscription() {
        let (messenger, client_id, _rx) = create_test_messenger().await;
        let task_id = Uuid::new_v4();

        // Subscribe to task
        let result = messenger.subscribe_to_task(client_id, task_id).await;
        assert!(result.is_ok());

        // Unsubscribe from task
        let result = messenger.unsubscribe_from_task(client_id, task_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_authentication() {
        let (messenger, client_id, _rx) = create_test_messenger().await;

        // Initially not authenticated
        assert_eq!(messenger.authenticated_client_count().await, 0);

        // Authenticate
        let user = CurrentUser {
            email: "test@example.com".to_string(),
            name: "test_user".to_string(),
            picture: None,
            access_token: None,
        };

        let result = messenger.authenticate_client(client_id, user).await;
        assert!(result.is_ok());

        // Now authenticated
        assert_eq!(messenger.authenticated_client_count().await, 1);
    }
}
