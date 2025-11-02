use anyhow::Context;
use futures_util::StreamExt;
use owo_colors::OwoColorize;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, warn};

use crate::{cli::LogsCommand, context::AppContext, utils::status_line::Status};
use scotty_core::websocket::message::WebSocketMessage;
use scotty_types::{LogStreamRequest, OutputLine};
use uuid::Uuid;

/// View logs for an app service
pub async fn logs_app(context: &AppContext, cmd: &LogsCommand) -> anyhow::Result<()> {
    // Validate app and service using shared utility
    let _app_data =
        super::validate_app_and_service(context, &cmd.app_name, &cmd.service_name, "app:logs")
            .await?;

    // Use unified WebSocket approach for both historical and real-time logs
    stream_logs_websocket(context, cmd).await
}

/// Stream logs using WebSocket-only approach for both historical and real-time logs
async fn stream_logs_websocket(context: &AppContext, cmd: &LogsCommand) -> anyhow::Result<()> {
    use crate::websocket::AuthenticatedWebSocket;

    let ui = context.ui();

    let stream_type = if cmd.follow {
        "real-time"
    } else {
        "historical"
    };
    ui.new_status_line(format!("Starting {} log stream...", stream_type));

    // Connect and authenticate to WebSocket
    ui.new_status_line("Connecting to WebSocket...");
    let mut ws = AuthenticatedWebSocket::connect(context.server())
        .await
        .context("Failed to connect to WebSocket")?;

    ui.success("🔐 WebSocket authenticated");

    // Create the log stream request
    let log_request = LogStreamRequest {
        app_name: cmd.app_name.clone(),
        service_name: cmd.service_name.clone(),
        follow: cmd.follow,
        lines: cmd.lines.map(|n| n as u32),
        since: cmd.since.clone(),
        until: cmd.until.clone(),
        timestamps: cmd.timestamps, // Simple flag: present = true, absent = false
    };

    // Send StartLogStream message
    let start_message = WebSocketMessage::StartLogStream(log_request);
    ws.send(start_message)
        .await
        .context("Failed to send log stream request")?;

    ui.new_status_line(format!(
        "Requesting {} logs for {} service...",
        stream_type,
        cmd.service_name.yellow()
    ));

    let display_message = if cmd.follow {
        format!(
            "Following logs for {} service in {} app (Press Ctrl+C to stop)...",
            cmd.service_name.yellow(),
            cmd.app_name.yellow()
        )
    } else {
        match cmd.lines {
            Some(n) => format!(
                "Fetching {} lines of logs for {} service in {} app...",
                n,
                cmd.service_name.yellow(),
                cmd.app_name.yellow()
            ),
            None => format!(
                "Fetching all available logs for {} service in {} app...",
                cmd.service_name.yellow(),
                cmd.app_name.yellow()
            ),
        }
    };
    ui.set_status(&display_message, Status::Running);

    let mut logs_received = false;
    let mut current_stream_id: Option<Uuid> = None;

    // Listen for WebSocket messages
    while let Some(message) = ws.receiver.next().await {
        match message {
            Ok(Message::Text(text)) => {
                if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                    match ws_message {
                        WebSocketMessage::LogsStreamStarted(info) => {
                            current_stream_id = Some(info.stream_id);
                        }
                        WebSocketMessage::LogsStreamData(data) => {
                            logs_received = true;
                            for line in data.lines {
                                display_log_line(&line, cmd, ui);
                            }
                        }
                        WebSocketMessage::LogsStreamEnded(end) => {
                            if !logs_received && !cmd.follow {
                                ui.println(
                                    "No logs available for the specified criteria"
                                        .yellow()
                                        .to_string(),
                                );
                            } else if cmd.follow {
                                ui.println(
                                    format!("Log stream ended: {}", end.reason)
                                        .yellow()
                                        .to_string(),
                                );
                            }
                            // For historical logs that were received, just complete silently without status message
                            // Always break on stream ended - server has delivered all logs
                            break;
                        }
                        WebSocketMessage::LogsStreamError(error) => {
                            ui.set_status(
                                &format!("Log streaming failed: {}", error),
                                Status::Failed,
                            );
                            break;
                        }
                        WebSocketMessage::Error(msg) => {
                            ui.set_status(&format!("WebSocket error: {}", msg), Status::Failed);
                            break;
                        }
                        _ => {
                            // Ignore other message types
                        }
                    }
                } else {
                    warn!("Unexpected WebSocket message: {}", text);
                }
            }
            Ok(Message::Close(_)) => {
                warn!("WebSocket connection closed unexpectedly");
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {
                // Ignore other message types (binary, ping, pong)
            }
        }
    }

    // Send stop message if we have a stream ID and it's a follow stream
    if let Some(stream_id) = current_stream_id {
        if cmd.follow {
            let stop_message = WebSocketMessage::StopLogStream { stream_id };
            let _ = ws.send(stop_message).await;
        }
    }

    if !logs_received && !cmd.follow {
        ui.println(
            "No historical logs found. Try using --follow for real-time streaming."
                .yellow()
                .to_string(),
        );
    }

    // Close the WebSocket connection properly
    let _ = ws.close().await;

    Ok(())
}

/// Display a single log line with formatting
fn display_log_line(line: &OutputLine, cmd: &LogsCommand, ui: &crate::utils::ui::Ui) {
    let show_timestamps = cmd.timestamps;

    let timestamp_str = if show_timestamps {
        line.timestamp.format("%Y-%m-%d %H:%M:%S%.3f").to_string()
    } else {
        String::new()
    };

    // Use UI helper to ensure proper display even with status lines
    // Trim trailing newline from content since ui.println adds one
    let content = line.content.trim_end_matches('\n');
    let formatted_line = if ui.is_terminal() {
        match line.stream {
            scotty_core::output::unified_output::OutputStreamType::Stdout => {
                format!("{} {}", timestamp_str.dimmed(), content)
            }
            scotty_core::output::unified_output::OutputStreamType::Stderr => {
                format!("{} {}", timestamp_str.dimmed(), content.dimmed())
            }
            scotty_core::output::unified_output::OutputStreamType::Status => {
                format!("{} [STATUS] {}", timestamp_str.dimmed(), content.green())
            }
            scotty_core::output::unified_output::OutputStreamType::StatusError => {
                format!(
                    "{} [ERROR] {}",
                    timestamp_str.dimmed(),
                    content.red().bold()
                )
            }
            scotty_core::output::unified_output::OutputStreamType::Progress => {
                format!("{} [PROGRESS] {}", timestamp_str.dimmed(), content.blue())
            }
            scotty_core::output::unified_output::OutputStreamType::Info => {
                format!("{} [INFO] {}", timestamp_str.dimmed(), content.cyan())
            }
        }
    } else {
        format!("{} {}", timestamp_str, content)
    };

    ui.println(formatted_line);
}
