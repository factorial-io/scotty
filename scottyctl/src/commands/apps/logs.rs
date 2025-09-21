use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use owo_colors::OwoColorize;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    api::{get, get_auth_token},
    cli::LogsCommand,
    context::AppContext,
    utils::status_line::Status,
};
use scotty_core::{apps::app_data::AppData, output::unified_output::OutputLine};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// WebSocket message types for client communication - matches server definitions
#[derive(Serialize, Deserialize, Debug)]
pub enum WebSocketMessage {
    // Authentication messages
    Authenticate { token: String },
    AuthenticationSuccess,
    AuthenticationFailed { reason: String },

    // Client → Server messages (require authentication)
    StartLogStream(LogStreamRequest),
    StopLogStream { stream_id: Uuid },

    // Server → Client messages
    LogsStreamStarted(LogsStreamInfo),
    LogsStreamData(LogsStreamData),
    LogsStreamEnded(LogsStreamEnd),
    LogsStreamError(LogsStreamError),
    Error(String),

    // Other message types we might receive
    AppListUpdated,
    AppInfoUpdated(String),
    TaskListUpdated,
    Ping,
    Pong,
}

/// Request to start a log stream with parameters
#[derive(Serialize, Deserialize, Debug)]
pub struct LogStreamRequest {
    pub app_name: String,
    pub service_name: String,
    pub follow: bool,
    pub lines: Option<u32>, // Number of lines for historical logs (default 100)
    pub since: Option<String>, // Time filter: "1h", "30m", or ISO timestamp
    pub until: Option<String>, // End time filter: ISO timestamp
    pub timestamps: bool,   // Include timestamps in output (flag: present=true, absent=false)
}

/// Information about a started log stream
#[derive(Serialize, Deserialize, Debug)]
pub struct LogsStreamInfo {
    pub stream_id: Uuid,
    pub app_name: String,
    pub service_name: String,
    pub follow: bool,
}

/// Log data from a stream
#[derive(Serialize, Deserialize, Debug)]
pub struct LogsStreamData {
    pub stream_id: Uuid,
    pub lines: Vec<OutputLine>,
}

/// Log stream ended notification
#[derive(Serialize, Deserialize, Debug)]
pub struct LogsStreamEnd {
    pub stream_id: Uuid,
    pub reason: String,
}

/// Log stream error notification
#[derive(Serialize, Deserialize, Debug)]
pub struct LogsStreamError {
    pub stream_id: Uuid,
    pub error: String,
}

/// View logs for an app service
pub async fn logs_app(context: &AppContext, cmd: &LogsCommand) -> anyhow::Result<()> {
    let ui = context.ui();

    // First validate that the app and service exist
    ui.new_status_line(format!(
        "Validating app {} and service {}...",
        cmd.app_name.yellow(),
        cmd.service_name.yellow()
    ));

    // Get app info and validate service exists
    let app_data = match get_app_data(context, &cmd.app_name).await {
        Ok(data) => data,
        Err(e) => {
            ui.failed(format!("Failed to get app information: {}", e));
            return Err(e);
        }
    };

    // Check if the requested service exists
    let available_services: Vec<String> = app_data
        .services
        .iter()
        .map(|s| s.service.clone())
        .collect();

    if !available_services.contains(&cmd.service_name) {
        ui.failed(format!(
            "Service '{}' not found in app '{}'",
            cmd.service_name.red(),
            cmd.app_name.yellow()
        ));

        // Show available services in a nice format
        ui.println("");
        ui.println(format!("Available services in {}:", cmd.app_name.yellow()));

        for service in &app_data.services {
            let status_icon = if service.is_running() {
                "✓".green().to_string()
            } else {
                "✗".red().to_string()
            };
            ui.println(format!(
                "  {} {} ({})",
                status_icon,
                service.service.green(),
                service.status.to_string().dimmed()
            ));
        }

        ui.println("");
        ui.println(format!(
            "Usage: {} app:logs {} <service_name>",
            "scottyctl".cyan(),
            cmd.app_name
        ));

        return Err(anyhow::anyhow!(
            "Service '{}' not found. Please choose from the available services listed above.",
            cmd.service_name
        ));
    }

    ui.success(format!(
        "Found service {} in app {}",
        cmd.service_name.yellow(),
        cmd.app_name.yellow()
    ));

    // Use unified WebSocket approach for both historical and real-time logs
    stream_logs_websocket(context, cmd).await
}

/// Get app data for validation
async fn get_app_data(context: &AppContext, app_name: &str) -> anyhow::Result<AppData> {
    // Get app info to validate app exists and get available services
    let result = get(context.server(), &format!("apps/info/{}", app_name))
        .await
        .with_context(|| format!("App '{}' not found or not accessible", app_name))?;

    let app_data: AppData =
        serde_json::from_value(result).context("Failed to parse app information")?;

    Ok(app_data)
}

/// Stream logs using WebSocket-only approach for both historical and real-time logs
async fn stream_logs_websocket(context: &AppContext, cmd: &LogsCommand) -> anyhow::Result<()> {
    let ui = context.ui();

    let stream_type = if cmd.follow {
        "real-time"
    } else {
        "historical"
    };
    ui.new_status_line(format!("Starting {} log stream...", stream_type));

    // Connect to WebSocket first (unauthenticated)
    let ws_url = context
        .server()
        .server
        .replace("http://", "ws://")
        .replace("https://", "wss://");
    let ws_url = format!("{}/ws", ws_url);

    ui.new_status_line("Connecting to WebSocket...");
    let (ws_stream, _) = connect_async(&ws_url)
        .await
        .context("Failed to connect to WebSocket")?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    ui.success("Connected to WebSocket");

    // Get authentication token and send authentication message
    let token = get_auth_token(context.server())
        .await
        .context("Failed to get authentication token")?;

    ui.new_status_line("Authenticating WebSocket connection...");

    let auth_message = WebSocketMessage::Authenticate { token };
    let auth_json = serde_json::to_string(&auth_message)
        .context("Failed to serialize authentication message")?;

    ws_sender
        .send(Message::Text(auth_json.into()))
        .await
        .context("Failed to send authentication message")?;

    // Wait for authentication response
    let mut authenticated = false;
    while let Some(message) = ws_receiver.next().await {
        match message {
            Ok(Message::Text(text)) => {
                if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                    match ws_message {
                        WebSocketMessage::AuthenticationSuccess => {
                            ui.success("🔐 WebSocket authentication successful");
                            authenticated = true;
                            break;
                        }
                        WebSocketMessage::AuthenticationFailed { reason } => {
                            return Err(anyhow::anyhow!(
                                "WebSocket authentication failed: {}",
                                reason
                            ));
                        }
                        WebSocketMessage::Error(msg) => {
                            return Err(anyhow::anyhow!(
                                "WebSocket error during authentication: {}",
                                msg
                            ));
                        }
                        _ => {
                            // Ignore other messages during authentication
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                return Err(anyhow::anyhow!(
                    "WebSocket connection closed during authentication"
                ));
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "WebSocket error during authentication: {}",
                    e
                ));
            }
            _ => {
                // Ignore other message types
            }
        }
    }

    if !authenticated {
        return Err(anyhow::anyhow!("WebSocket authentication timed out"));
    }

    // Create the log stream request
    let log_request = LogStreamRequest {
        app_name: cmd.app_name.clone(),
        service_name: cmd.service_name.clone(),
        follow: cmd.follow,
        lines: Some(cmd.lines as u32),
        since: cmd.since.clone(),
        until: cmd.until.clone(),
        timestamps: cmd.timestamps, // Simple flag: present = true, absent = false
    };

    // Send StartLogStream message
    let start_message = WebSocketMessage::StartLogStream(log_request);
    let message_json =
        serde_json::to_string(&start_message).context("Failed to serialize log stream request")?;

    ws_sender
        .send(Message::Text(message_json.into()))
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
        format!(
            "Fetching {} lines of logs for {} service in {} app...",
            cmd.lines,
            cmd.service_name.yellow(),
            cmd.app_name.yellow()
        )
    };
    ui.println(display_message);

    let mut logs_received = false;
    let mut current_stream_id: Option<Uuid> = None;

    // Listen for WebSocket messages
    while let Some(message) = ws_receiver.next().await {
        match message {
            Ok(Message::Text(text)) => {
                if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                    match ws_message {
                        WebSocketMessage::LogsStreamStarted(info) => {
                            current_stream_id = Some(info.stream_id);

                            // Update status line to show active streaming
                            let status_message = if cmd.follow {
                                format!(
                                    "Streaming logs from {} service in real-time...",
                                    info.service_name.yellow()
                                )
                            } else {
                                format!(
                                    "Fetching logs from {} service...",
                                    info.service_name.yellow()
                                )
                            };
                            ui.set_status(&status_message, Status::Running);

                            if cmd.follow {
                                ui.println(
                                    format!(
                                        "📡 Started real-time log stream {} for {}:{}",
                                        info.stream_id, info.app_name, info.service_name
                                    )
                                    .green()
                                    .to_string(),
                                );
                            }
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
                                    "📄 No logs available for the specified criteria"
                                        .yellow()
                                        .to_string(),
                                );
                            } else if cmd.follow {
                                ui.println(
                                    format!("📄 Log stream ended: {}", end.reason)
                                        .yellow()
                                        .to_string(),
                                );
                            }
                            // For historical logs that were received, just complete silently without status message
                            // Always break on stream ended - server has delivered all logs
                            break;
                        }
                        WebSocketMessage::LogsStreamError(error) => {
                            ui.set_status("Log streaming failed", Status::Failed);
                            ui.eprintln(
                                format!("❌ Log stream error: {}", error.error)
                                    .red()
                                    .to_string(),
                            );
                            break;
                        }
                        WebSocketMessage::Error(msg) => {
                            ui.set_status("WebSocket error", Status::Failed);
                            ui.eprintln(format!("❌ WebSocket error: {}", msg).red().to_string());
                            break;
                        }
                        _ => {
                            // Ignore other message types
                        }
                    }
                } else {
                    // Log unexpected messages for debugging
                    ui.println(format!("🔍 Unexpected WebSocket message: {}", text));
                }
            }
            Ok(Message::Close(_)) => {
                ui.println(
                    "📡 WebSocket connection closed unexpectedly"
                        .yellow()
                        .to_string(),
                );
                break;
            }
            Err(e) => {
                ui.eprintln(format!("🚫 WebSocket error: {}", e).red().to_string());
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
            if let Ok(message_json) = serde_json::to_string(&stop_message) {
                let _ = ws_sender.send(Message::Text(message_json.into())).await;
            }
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
    let _ = ws_sender.close().await;

    Ok(())
}

/// Display a single log line with formatting
fn display_log_line(line: &OutputLine, cmd: &LogsCommand, ui: &crate::utils::ui::Ui) {
    let show_timestamps = cmd.timestamps;

    let timestamp_str = if show_timestamps {
        format!(
            "{} ",
            line.timestamp
                .format("%Y-%m-%d %H:%M:%S%.3f")
                .to_string()
                .dimmed()
        )
    } else {
        String::new()
    };

    let stream_prefix = match line.stream {
        scotty_core::output::unified_output::OutputStreamType::Stdout => String::new(),
        scotty_core::output::unified_output::OutputStreamType::Stderr => {
            format!("{} ", "[STDERR]".red())
        }
    };

    // Use UI helper to ensure proper display even with status lines
    // Trim trailing newline from content since ui.println adds one
    let content = line.content.trim_end_matches('\n');
    ui.println(format!("{}{}{}", timestamp_str, stream_prefix, content));
}
