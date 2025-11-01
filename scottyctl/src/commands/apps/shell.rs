use anyhow::Context;
use futures_util::StreamExt;
use owo_colors::OwoColorize;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, warn};

use crate::{api::post, cli::ShellCommand, context::AppContext};
use scotty_core::websocket::message::WebSocketMessage;
use uuid::Uuid;

/// Open an interactive shell for an app service
pub async fn shell_app(context: &AppContext, cmd: &ShellCommand) -> anyhow::Result<()> {
    // Validate app and service using shared utility
    let _app_data = super::validate_app_and_service(
        context,
        &cmd.app_name,
        &cmd.service_name,
        "app:shell",
    )
    .await?;

    // Create shell session and open interactive terminal
    open_shell(context, cmd).await
}

/// Create shell session and open interactive terminal
async fn open_shell(context: &AppContext, cmd: &ShellCommand) -> anyhow::Result<()> {
    use crate::websocket::AuthenticatedWebSocket;
    use serde::{Deserialize, Serialize};

    let ui = context.ui();

    ui.new_status_line("Creating shell session...");

    // Create the shell session via REST API
    #[derive(Serialize)]
    struct CreateShellRequest {
        shell_command: Option<String>,
    }

    #[derive(Deserialize)]
    struct CreateShellResponse {
        session_id: Uuid,
        #[allow(dead_code)]
        message: String,
    }

    // Build shell command: either custom shell, or bash -c "command", or just bash
    let shell_command = if let Some(command) = &cmd.command {
        let shell = cmd.shell.as_deref().unwrap_or("/bin/bash");
        Some(format!("{} -c '{}'", shell, command.replace('\'', "'\\''")))
    } else {
        cmd.shell.clone()
    };

    let request = CreateShellRequest { shell_command };
    let request_value = serde_json::to_value(&request)?;

    let response_value = post(
        context.server(),
        &format!(
            "apps/{}/services/{}/shell",
            cmd.app_name, cmd.service_name
        ),
        request_value,
    )
    .await
    .context("Failed to create shell session")?;

    let response: CreateShellResponse = serde_json::from_value(response_value)
        .context("Failed to parse shell session response")?;

    let session_id = response.session_id;
    ui.success(format!("Shell session created: {}", session_id));

    // Connect to WebSocket
    ui.new_status_line("Connecting to WebSocket...");
    let mut ws = AuthenticatedWebSocket::connect(context.server())
        .await
        .context("Failed to connect to WebSocket")?;

    ui.success("🔐 WebSocket authenticated");

    // TODO: Implement terminal raw mode and bidirectional I/O
    ui.println(format!(
        "{}",
        "Interactive shell not yet fully implemented".yellow()
    ));
    ui.println(format!(
        "Session ID: {} - waiting for messages...",
        session_id
    ));

    // For now, just listen to messages
    while let Some(message) = ws.receiver.next().await {
        match message {
            Ok(Message::Text(text)) => {
                if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                    match ws_message {
                        WebSocketMessage::ShellSessionCreated(info) => {
                            ui.println(format!(
                                "Shell session confirmed: {} on {}/{}",
                                info.session_id, info.app_name, info.service_name
                            ));
                        }
                        WebSocketMessage::ShellSessionData(data) => {
                            // Print shell output
                            print!("{}", data.data);
                        }
                        WebSocketMessage::ShellSessionEnded(end) => {
                            ui.println(
                                format!("Shell session ended: {}", end.reason)
                                    .yellow()
                                    .to_string(),
                            );
                            break;
                        }
                        WebSocketMessage::ShellSessionError(error) => {
                            ui.println(
                                format!("Shell error: {}", error.error).red().to_string(),
                            );
                            break;
                        }
                        WebSocketMessage::Error(msg) => {
                            ui.println(
                                format!("WebSocket error: {}", msg).red().to_string(),
                            );
                            break;
                        }
                        _ => {
                            // Ignore other message types
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                warn!("WebSocket connection closed");
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {
                // Ignore other message types
            }
        }
    }

    // Close the WebSocket connection
    let _ = ws.close().await;

    Ok(())
}
