use tracing::instrument;
use uuid::Uuid;

use crate::api::message::WebSocketMessage;
use crate::app_state::SharedAppState;

use super::ws::{broadcast_message, send_message};

#[instrument(skip(state))]
pub async fn handle_websocket_message(
    state: &SharedAppState,
    client_id: Uuid,
    msg: &WebSocketMessage,
) {
    match msg {
        WebSocketMessage::Ping => {
            send_message(state, client_id, WebSocketMessage::Pong).await;
        }
        WebSocketMessage::Pong => {}
        WebSocketMessage::Error(_) => {}
    }
}
