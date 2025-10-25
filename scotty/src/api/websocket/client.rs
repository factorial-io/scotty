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
    }).await;

    while let Some(Ok(msg)) = receiver.next().await {
        info!("Received message: {:?}", msg);
        match msg {
            Message::Text(text) => {
                if let Ok(parsed_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                    handle_websocket_message(&state, client_id, &parsed_msg).await;
                } else {
                    state
                        .messenger
                        .broadcast_to_all(WebSocketMessage::Error(
                            "Could not parse message".to_string(),
                        ))
                        .await;
                }
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

        // Stop any active log streams for this client
        cleanup_client_streams(&state, client_id).await;

        // Remove client from the list
        state.messenger.remove_client(client_id).await;

        info!("WebSocket client {} cleanup completed", client_id);
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

/// Clean up all streams associated with a disconnected client
async fn cleanup_client_streams(state: &SharedAppState, client_id: Uuid) {
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
}
