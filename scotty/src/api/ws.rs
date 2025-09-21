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
use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::api::message_handler::handle_websocket_message;
use crate::{api::message::WebSocketMessage, app_state::SharedAppState};

#[instrument(skip(state, ws))]
async fn websocket_handler(ws: WebSocket, state: SharedAppState, client_id: Uuid) {
    let (mut sender, mut receiver) = ws.split();
    let (tx, mut rx) = broadcast::channel(1000); // Increase buffer for log streaming

    {
        info!("New WebSocket connection");
        let state = state.clone();
        let mut clients = state.clients.lock().await;
        clients.insert(
            client_id,
            crate::app_state::WebSocketClient::new(tx.clone()),
        );
    }

    // Don't send initial ping - wait for authentication first

    tokio::spawn(async move {
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
        info!("Received message: {:?}", msg);
        match msg {
            Message::Text(text) => {
                if let Ok(parsed_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                    handle_websocket_message(&state, client_id, &parsed_msg).await;
                } else {
                    broadcast_message(
                        &state,
                        WebSocketMessage::Error("Could not parse message".to_string()),
                    )
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
        let mut clients = state.clients.lock().await;
        clients.remove(&client_id);

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

pub async fn broadcast_message(state: &SharedAppState, msg: WebSocketMessage) {
    let state = state.clone();
    let clients = state.clients.lock().await;
    let serialized_msg = serde_json::to_string(&msg).expect("Failed to serialize message");
    for client in clients.values() {
        let _ = client
            .sender
            .send(Message::Text(serialized_msg.clone().into()));
    }
}

pub async fn send_message(state: &SharedAppState, uuid: Uuid, msg: WebSocketMessage) {
    let state_clone = state.clone();
    let clients = state_clone.clients.lock().await;
    let serialized_msg = serde_json::to_string(&msg).expect("Failed to serialize message");
    if let Some(client) = clients.get(&uuid) {
        if let Err(e) = client.sender.send(Message::Text(serialized_msg.into())) {
            warn!("Failed to queue message for client {}: {:?}", uuid, e);
        }
    } else {
        // Client not found - stream cleanup is handled proactively during disconnect
        warn!(
            "Failed to send message to client {}: client not found",
            uuid
        );
    }
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
