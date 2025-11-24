use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use scotty_core::websocket::message::WebSocketMessage;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream};

use crate::api::get_auth_token;
use crate::context::ServerSettings;

type WsSender = futures_util::stream::SplitSink<
    WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;

type WsReceiver = futures_util::stream::SplitStream<
    WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
>;

/// WebSocket connection that has been authenticated
pub struct AuthenticatedWebSocket {
    pub sender: WsSender,
    pub receiver: WsReceiver,
}

impl AuthenticatedWebSocket {
    /// Connect and authenticate to the WebSocket endpoint
    pub async fn connect(server: &ServerSettings) -> anyhow::Result<Self> {
        // Build WebSocket URL from server URL
        let ws_url = build_websocket_url(&server.server)?;

        // Connect to WebSocket
        let (ws_stream, _) = connect_async(&ws_url)
            .await
            .context("Failed to connect to WebSocket")?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Get authentication token and send authentication message
        let token = get_auth_token(server).await?;
        let auth_message = WebSocketMessage::Authenticate { token };
        let auth_json = serde_json::to_string(&auth_message)?;
        ws_sender.send(Message::Text(auth_json.into())).await?;

        // Wait for authentication response
        let mut authenticated = false;
        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                        match ws_message {
                            WebSocketMessage::AuthenticationSuccess => {
                                authenticated = true;
                                break;
                            }
                            WebSocketMessage::AuthenticationFailed { reason } => {
                                return Err(anyhow::anyhow!(
                                    "WebSocket authentication failed: {}",
                                    reason
                                ));
                            }
                            _ => {} // Ignore other messages during authentication
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    return Err(anyhow::anyhow!("WebSocket closed during authentication"));
                }
                Err(e) => return Err(anyhow::anyhow!("WebSocket error: {}", e)),
                _ => {}
            }
        }

        if !authenticated {
            return Err(anyhow::anyhow!("WebSocket authentication timed out"));
        }

        Ok(Self {
            sender: ws_sender,
            receiver: ws_receiver,
        })
    }

    /// Send a WebSocket message
    pub async fn send(&mut self, message: WebSocketMessage) -> anyhow::Result<()> {
        let message_json = serde_json::to_string(&message)?;
        self.sender
            .send(Message::Text(message_json.into()))
            .await
            .context("Failed to send WebSocket message")
    }

    /// Close the WebSocket connection
    pub async fn close(mut self) -> anyhow::Result<()> {
        self.sender
            .close()
            .await
            .context("Failed to close WebSocket connection")
    }

    /// Split the WebSocket into sender and receiver parts
    pub fn split(self) -> (WsSender, WsReceiver) {
        (self.sender, self.receiver)
    }
}

/// Build WebSocket URL from server URL
pub fn build_websocket_url(server_url: &str) -> anyhow::Result<String> {
    let ws_url = server_url
        .replace("http://", "ws://")
        .replace("https://", "wss://");
    Ok(format!("{}/ws", ws_url))
}
