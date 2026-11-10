use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::SinkExt;
use futures_util::StreamExt;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, OwnedSemaphorePermit, Semaphore};
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

    let dispatcher = MessageDispatcher::new(state.clone(), client_id);
    while let Some(Ok(msg)) = receiver.next().await {
        // Never log the content: it carries shell input (passwords, piped files).
        debug!("Received WebSocket message from client {}", client_id);
        match msg {
            Message::Text(text) => {
                crate::metrics::websocket::record_message_received();
                dispatcher.dispatch(&text).await;
            }
            Message::Binary(_bin) => {
                // not handled.
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    // Input still queued for a shell is of no use once the client is gone.
    dispatcher.abort();

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

/// Bytes of shell input a client may have queued before reading from its
/// connection pauses.
const SHELL_INPUT_BUFFER_BYTES: usize = 1024 * 1024;

/// Handles the messages of one client.
///
/// Shell input (`ShellSessionData`, `CloseShellStdin`) goes through a queue
/// served by its own task, in order. When a process does not read its stdin,
/// only that queue waits; the connection keeps being read, so `Terminate`,
/// `Resize`, pings and other sessions still get through. Reading pauses only
/// once [`SHELL_INPUT_BUFFER_BYTES`] of input are queued, which slows the
/// client down instead of buffering without limit.
struct MessageDispatcher {
    state: SharedAppState,
    client_id: Uuid,
    input_budget: Arc<Semaphore>,
    input_queue: mpsc::UnboundedSender<(WebSocketMessage, OwnedSemaphorePermit)>,
    input_task: tokio::task::JoinHandle<()>,
}

impl MessageDispatcher {
    fn new(state: SharedAppState, client_id: Uuid) -> Self {
        let (input_queue, mut queued) =
            mpsc::unbounded_channel::<(WebSocketMessage, OwnedSemaphorePermit)>();
        let task_state = state.clone();
        let input_task = tokio::spawn(async move {
            // The permit is released once the input reached the session.
            while let Some((msg, _permit)) = queued.recv().await {
                handle_websocket_message(&task_state, client_id, &msg).await;
            }
        });
        Self {
            state,
            client_id,
            input_budget: Arc::new(Semaphore::new(SHELL_INPUT_BUFFER_BYTES)),
            input_queue,
            input_task,
        }
    }

    async fn dispatch(&self, text: &str) {
        let msg = match serde_json::from_str::<WebSocketMessage>(text) {
            Ok(msg) => msg,
            Err(_) => {
                self.state
                    .messenger
                    .send_error(self.client_id, "Could not parse message".to_string())
                    .await;
                return;
            }
        };
        if !matches!(
            msg,
            WebSocketMessage::ShellSessionData(_) | WebSocketMessage::CloseShellStdin { .. }
        ) {
            handle_websocket_message(&self.state, self.client_id, &msg).await;
            return;
        }
        // A message larger than the whole budget takes all of it.
        let cost = text.len().clamp(1, SHELL_INPUT_BUFFER_BYTES) as u32;
        let Ok(permit) = self.input_budget.clone().acquire_many_owned(cost).await else {
            return;
        };
        if self.input_queue.send((msg, permit)).is_err() {
            warn!(
                "Shell input task of client {} has stopped, dropping input",
                self.client_id
            );
        }
    }

    fn abort(&self) {
        self.input_task.abort();
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

        MessageDispatcher::new(state.clone(), sender_id)
            .dispatch("not json")
            .await;

        let Message::Text(text) = sender_rx.try_recv().unwrap() else {
            panic!("expected a text message");
        };
        assert!(matches!(
            serde_json::from_str(&text).unwrap(),
            WebSocketMessage::Error(e) if e == "Could not parse message"
        ));
        assert!(other_rx.try_recv().is_err());
    }

    /// A process that does not read stdin fills its stdin queue. Further
    /// input waits, but control messages of the same client must still get
    /// through, or the session could not be terminated.
    #[tokio::test]
    async fn terminate_gets_through_while_stdin_is_full() {
        use crate::docker::services::shell::ShellCommand;
        use scotty_types::{ShellDataType, ShellSessionData};
        use std::time::Duration;

        let state = crate::api::test_utils::create_test_app_state_with_config(
            "tests/test_bearer_auth",
            None,
        )
        .await;
        let client_id = Uuid::new_v4();
        let (tx, _rx) = broadcast::channel(64);
        state
            .messenger
            .add_client(client_id, WebSocketClient::new(tx))
            .await;
        state
            .messenger
            .authenticate_client(
                client_id,
                crate::api::auth_core::CurrentUser {
                    email: "owner@example.com".to_string(),
                    name: "owner".to_string(),
                    picture: None,
                    access_token: None,
                },
            )
            .await
            .unwrap();
        let (session_id, mut control, mut stdin) = state
            .shell_service
            .insert_test_session_with_stdin_queue("app", client_id, 1)
            .await;

        let dispatcher = MessageDispatcher::new(state.clone(), client_id);
        let input = serde_json::to_string(&WebSocketMessage::ShellSessionData(
            ShellSessionData::text(session_id, ShellDataType::Input, "x"),
        ))
        .unwrap();
        // More input than the stdin queue holds; nobody reads it.
        for _ in 0..4 {
            tokio::time::timeout(Duration::from_secs(1), dispatcher.dispatch(&input))
                .await
                .expect("queuing input must not wait for the process");
        }
        let terminate =
            serde_json::to_string(&WebSocketMessage::TerminateShellSession { session_id }).unwrap();
        tokio::time::timeout(Duration::from_secs(1), dispatcher.dispatch(&terminate))
            .await
            .expect("terminate must not wait behind input");
        assert!(matches!(control.try_recv(), Ok(ShellCommand::Terminate)));

        // The queued input still arrives, in order, once stdin is read.
        for _ in 0..4 {
            let cmd = tokio::time::timeout(Duration::from_secs(1), stdin.recv())
                .await
                .unwrap();
            assert!(matches!(cmd, Some(ShellCommand::Input(data)) if data == b"x"));
        }
        dispatcher.abort();
    }
}
