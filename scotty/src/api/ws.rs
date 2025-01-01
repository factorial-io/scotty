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
use tracing::{info, instrument};
use uuid::Uuid;

use crate::api::message_handler::handle_websocket_message;
use crate::{api::message::WebSocketMessage, app_state::SharedAppState};

#[instrument(skip(state, ws))]
async fn websocket_handler(ws: WebSocket, state: SharedAppState, client_id: Uuid) {
    let (mut sender, mut receiver) = ws.split();
    let (tx, mut rx) = broadcast::channel(16);

    {
        info!("New WebSocket connection");
        let state = state.clone();
        let mut clients = state.clients.lock().await;
        clients.insert(client_id, tx.clone());
    }

    send_message(&state, client_id, WebSocketMessage::Ping).await;

    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(msg.clone()).await.is_err() {
                break;
            }
        }
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
        info!("WebSocket disconnected");
        let state = state.clone();
        let mut clients = state.clients.lock().await;
        clients.remove(&client_id);
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
        let _ = client.send(Message::Text(serialized_msg.clone()));
    }
}

pub async fn send_message(state: &SharedAppState, uuid: Uuid, msg: WebSocketMessage) {
    let state = state.clone();
    let clients = state.clients.lock().await;
    let serialized_msg = serde_json::to_string(&msg).expect("Failed to serialize message");
    if let Some(client) = clients.get(&uuid) {
        let _ = client.send(Message::Text(serialized_msg));
    }
}
