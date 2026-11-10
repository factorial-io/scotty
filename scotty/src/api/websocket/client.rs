use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::SinkExt;
use futures_util::StreamExt;
use tokio::sync::broadcast;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::api::auth_core;
use crate::api::websocket::handlers::handle_websocket_message;
use crate::app_state::SharedAppState;
use scotty_core::settings::api_server::AuthMode;
use scotty_core::websocket::message::WebSocketMessage;

#[instrument(skip(state, ws))]
async fn websocket_handler(ws: WebSocket, state: SharedAppState, client_id: Uuid) {
    let (mut sender, mut receiver) = ws.split();
    let (tx, mut rx) = broadcast::channel(1000); // Increase buffer for log streaming

    {
        info!("New WebSocket connection");
        crate::metrics::websocket::record_connection_opened();
        let client = crate::app_state::WebSocketClient::new(tx.clone());
        state.messenger.add_client(client_id, client).await;
    }

    // Auto-authenticate in development mode
    if matches!(state.settings.api.auth_mode, AuthMode::Development) {
        debug!(
            "Auto-authenticating WebSocket client {} in development mode",
            client_id
        );
        let dev_user = auth_core::authenticate_dev_user(&state);
        if let Err(e) = state
            .messenger
            .authenticate_client(client_id, dev_user)
            .await
        {
            warn!(
                "Failed to auto-authenticate WebSocket client {} in dev mode: {}",
                client_id, e
            );
        } else {
            info!(
                "WebSocket client {} auto-authenticated in development mode",
                client_id
            );
            state.messenger.send_auth_success(client_id).await;
        }
    }

    // Don't send initial ping - wait for authentication first

    crate::metrics::spawn_instrumented(async move {
        info!("Started WebSocket forwarding task for client {}", client_id);
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if let Err(e) = sender.send(msg.clone()).await {
                        warn!("Failed to send message to WebSocket client {}: {:?}, closing forwarding task", client_id, e);
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!(
                        "Broadcast channel closed for client {}, ending forwarding task",
                        client_id
                    );
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(
                        "WebSocket forwarding task for client {} lagged by {} messages, continuing",
                        client_id, n
                    );
                    // Continue trying to receive more messages
                }
            }
        }
        info!("WebSocket forwarding task ended for client {}", client_id);
    });

    while let Some(Ok(msg)) = receiver.next().await {
        // Never log the content: it carries shell input (passwords, piped files).
        debug!("Received WebSocket message from client {}", client_id);
        match msg {
            Message::Text(text) => {
                crate::metrics::websocket::record_message_received();
                handle_text_message(&state, client_id, &text).await;
            }
            Message::Binary(_bin) => {
                // not handled.
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    {
        info!("WebSocket client {} disconnected, cleaning up", client_id);

        // Stop log streams and terminate shell sessions owned by this client
        cleanup_client_streams(&state, client_id).await;

        // Remove client from the list
        state.messenger.remove_client(client_id).await;

        crate::metrics::websocket::record_connection_closed();
        info!("WebSocket client {} cleanup completed", client_id);
    }
}

async fn handle_text_message(state: &SharedAppState, client_id: Uuid, text: &str) {
    match serde_json::from_str::<WebSocketMessage>(text) {
        Ok(parsed_msg) => handle_websocket_message(state, client_id, &parsed_msg).await,
        Err(_) => {
            state
                .messenger
                .send_error(client_id, "Could not parse message".to_string())
                .await
        }
    }
}

#[instrument(skip(ws, state))]
pub async fn ws_handler(
    State(state): State<SharedAppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    info!("WebSocket upgrade request");
    let client_id = Uuid::new_v4();

    ws.on_upgrade(move |ws| websocket_handler(ws, state, client_id))
}

/// Clean up all log streams and shell sessions owned by a disconnected client.
///
/// Task output streams need no cleanup here: they end on their own once the
/// client is removed from the messenger.
pub(crate) async fn cleanup_client_streams(state: &SharedAppState, client_id: Uuid) {
    info!("Cleaning up streams for disconnected client {}", client_id);

    // Stop all streams associated with this client using the shared service
    let stopped_stream_ids = state.logs_service.stop_client_streams(client_id).await;

    if !stopped_stream_ids.is_empty() {
        info!(
            "Stopped {} log streams for disconnected client {}: {:?}",
            stopped_stream_ids.len(),
            client_id,
            stopped_stream_ids
        );
    } else {
        info!(
            "No active log streams found for disconnected client {}",
            client_id
        );
    }

    let terminated_session_ids = state
        .shell_service
        .terminate_client_sessions(client_id)
        .await;
    if !terminated_session_ids.is_empty() {
        info!(
            "Terminated {} shell sessions for disconnected client {}",
            terminated_session_ids.len(),
            client_id
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::WebSocketClient;

    #[tokio::test]
    async fn parse_errors_go_only_to_the_sender() {
        let state = crate::api::test_utils::create_test_app_state_with_config(
            "tests/test_bearer_auth",
            None,
        )
        .await;
        let (sender_id, other_id) = (Uuid::new_v4(), Uuid::new_v4());
        let (sender_tx, mut sender_rx) = broadcast::channel(8);
        let (other_tx, mut other_rx) = broadcast::channel(8);
        state
            .messenger
            .add_client(sender_id, WebSocketClient::new(sender_tx))
            .await;
        state
            .messenger
            .add_client(other_id, WebSocketClient::new(other_tx))
            .await;

        handle_text_message(&state, sender_id, "not json").await;

        let Message::Text(text) = sender_rx.try_recv().unwrap() else {
            panic!("expected a text message");
        };
        assert!(matches!(
            serde_json::from_str(&text).unwrap(),
            WebSocketMessage::Error(e) if e == "Could not parse message"
        ));
        assert!(other_rx.try_recv().is_err());
    }
}
