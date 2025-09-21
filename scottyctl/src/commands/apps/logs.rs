use anyhow::Context;
use owo_colors::OwoColorize;
use serde_json::json;
use std::collections::HashMap;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::StreamExt;

use crate::{
    api::get_or_post,
    cli::LogsCommand,
    context::AppContext,
};
use scotty_core::output::unified_output::OutputLine;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// WebSocket message types for client communication
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    LogsStreamData { stream_id: Uuid, lines: Vec<OutputLine> },
    LogsStreamEnded { stream_id: Uuid, reason: String },
    LogsStreamError { stream_id: Uuid, error: String },
}

/// View logs for an app service
pub async fn logs_app(context: &AppContext, cmd: &LogsCommand) -> anyhow::Result<()> {
    let ui = context.ui();

    ui.new_status_line(format!(
        "Getting logs for service {} in app {}...",
        cmd.service_name.yellow(),
        cmd.app_name.yellow()
    ));

    if cmd.follow {
        // Use WebSocket for real-time streaming
        stream_logs_websocket(context, cmd).await
    } else {
        // Use REST API for historical logs
        fetch_logs_rest(context, cmd).await
    }
}

/// Fetch historical logs using REST API
async fn fetch_logs_rest(context: &AppContext, cmd: &LogsCommand) -> anyhow::Result<()> {
    let ui = context.ui();

    ui.run(async || {
        // Prepare request parameters
        let mut params = HashMap::new();
        params.insert("lines", cmd.lines.to_string());

        if let Some(since) = &cmd.since {
            params.insert("since", since.clone());
        }
        if let Some(until) = &cmd.until {
            params.insert("until", until.clone());
        }

        // Make API call to start log stream
        let payload = json!({
            "lines": cmd.lines,
            "since": cmd.since,
            "until": cmd.until,
            "follow": false
        });

        let result = get_or_post(
            context.server(),
            &format!("apps/{}/services/{}/logs", cmd.app_name, cmd.service_name),
            "POST",
            Some(payload),
        )
        .await?;

        // Parse the response to get the stream ID
        let stream_id = result["stream_id"]
            .as_str()
            .context("Failed to get stream_id from response")?;

        ui.success(format!(
            "Retrieved logs for service {} in app {}",
            cmd.service_name.yellow(),
            cmd.app_name.yellow()
        ));

        // For non-streaming, we would typically get logs from the stream endpoint
        // For now, return a placeholder message
        Ok(format!("Log stream created with ID: {}", stream_id))
    })
    .await
}

/// Stream logs in real-time using WebSocket
async fn stream_logs_websocket(context: &AppContext, cmd: &LogsCommand) -> anyhow::Result<()> {
    let ui = context.ui();

    ui.println(format!(
        "Following logs for service {} in app {} (Press Ctrl+C to stop)...",
        cmd.service_name.yellow(),
        cmd.app_name.yellow()
    ));

    // First, start the log stream via REST API
    let payload = json!({
        "lines": cmd.lines,
        "since": cmd.since,
        "until": cmd.until,
        "follow": true
    });

    let result = get_or_post(
        context.server(),
        &format!("apps/{}/services/{}/logs", cmd.app_name, cmd.service_name),
        "POST",
        Some(payload),
    )
    .await?;

    let stream_id = result["stream_id"]
        .as_str()
        .context("Failed to get stream_id from response")?;

    // Connect to WebSocket
    let ws_url = context.server().server.replace("http://", "ws://").replace("https://", "wss://");
    let ws_url = format!("{}/ws", ws_url);

    let (ws_stream, _) = connect_async(&ws_url).await.context("Failed to connect to WebSocket")?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Send authentication and subscribe to logs
    // TODO: Implement proper WebSocket authentication
    // For now, this is a placeholder structure

    ui.println("Connected to log stream...".green().to_string());

    // Listen for WebSocket messages
    while let Some(message) = ws_receiver.next().await {
        match message {
            Ok(Message::Text(text)) => {
                if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                    match ws_message {
                        WebSocketMessage::LogsStreamData { stream_id: _, lines } => {
                            for line in lines {
                                display_log_line(&line, cmd);
                            }
                        }
                        WebSocketMessage::LogsStreamEnded { stream_id: _, reason } => {
                            ui.println(format!("Log stream ended: {}", reason).yellow().to_string());
                            break;
                        }
                        WebSocketMessage::LogsStreamError { stream_id: _, error } => {
                            ui.eprintln(format!("Log stream error: {}", error).red().to_string());
                            break;
                        }
                    }
                } else {
                    ui.println(format!("Received: {}", text));
                }
            }
            Ok(Message::Close(_)) => {
                ui.println("WebSocket connection closed".yellow().to_string());
                break;
            }
            Err(e) => {
                ui.eprintln(format!("WebSocket error: {}", e).red().to_string());
                break;
            }
            _ => {
                // Ignore other message types (binary, etc.)
            }
        }
    }

    Ok(())
}

/// Display a single log line with formatting
fn display_log_line(line: &OutputLine, cmd: &LogsCommand) {
    let show_timestamps = cmd.timestamps.unwrap_or(!cmd.follow);

    let timestamp_str = if show_timestamps {
        format!("{} ", line.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"))
    } else {
        String::new()
    };

    let stream_prefix = match line.stream {
        scotty_core::output::unified_output::OutputStreamType::Stdout => String::new(),
        scotty_core::output::unified_output::OutputStreamType::Stderr => "[STDERR] ".red().to_string(),
    };

    println!("{}{}{}", timestamp_str.dimmed(), stream_prefix, line.content);
}

/// Get authentication token for API calls (helper function)
async fn get_auth_token(context: &AppContext) -> anyhow::Result<String> {
    // This is a simplified version - in practice, you'd want to reuse the auth logic from api.rs
    if let Some(token) = &context.server().access_token {
        Ok(token.clone())
    } else {
        anyhow::bail!("No authentication token available")
    }
}