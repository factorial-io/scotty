use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use bollard::Docker;
use bollard::container::LogOutput;
use bollard::query_parameters::LogsOptions;
use futures_util::StreamExt;
use tracing::{info, error};
use chrono::Utc;

use crate::api::message::{WebSocketMessage, LogsStreamInfo, LogsStreamData, LogsStreamEnd, LogsStreamError};
use crate::api::ws::broadcast_message;
use crate::app_state::SharedAppState;
use scotty_core::output::{OutputLine, OutputStreamType};
use scotty_core::apps::app_data::AppData;

/// Active log streams tracked by the service
#[derive(Debug, Clone)]
pub struct LogStreamSession {
    pub stream_id: Uuid,
    pub app_name: String,
    pub service_name: String,
    pub container_id: String,
    pub sender: mpsc::Sender<LogStreamCommand>,
}

/// Commands that can be sent to a log stream task
#[derive(Debug)]
pub enum LogStreamCommand {
    Stop,
}

/// Service for managing container log streams
#[derive(Clone)]
pub struct LogStreamingService {
    docker: Docker,
    active_streams: Arc<RwLock<HashMap<Uuid, LogStreamSession>>>,
}

impl LogStreamingService {
    pub fn new(docker: Docker) -> Self {
        Self {
            docker,
            active_streams: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start streaming logs from a container
    pub async fn start_stream(
        &self,
        app_state: &SharedAppState,
        app_data: &AppData,
        service_name: &str,
        follow: bool,
        tail: Option<String>,
    ) -> Result<Uuid, String> {
        // Find the container for the service
        let container_state = app_data.services
            .iter()
            .find(|s| s.service == service_name)
            .ok_or_else(|| format!("Service '{}' not found in app '{}'", service_name, app_data.name))?;

        let container_id = container_state.id
            .as_ref()
            .ok_or_else(|| format!("Service '{}' has no container ID", service_name))?;

        // Generate stream ID
        let stream_id = Uuid::new_v4();

        // Create channel for controlling the stream
        let (tx, mut rx) = mpsc::channel::<LogStreamCommand>(1);

        // Create session info
        let session = LogStreamSession {
            stream_id,
            app_name: app_data.name.clone(),
            service_name: service_name.to_string(),
            container_id: container_id.clone(),
            sender: tx,
        };

        // Store session
        {
            let mut streams = self.active_streams.write().await;
            streams.insert(stream_id, session.clone());
        }

        // Send stream started message
        broadcast_message(
            app_state,
            WebSocketMessage::LogsStreamStarted(LogsStreamInfo {
                stream_id,
                app_name: app_data.name.clone(),
                service_name: service_name.to_string(),
                follow,
            }),
        ).await;

        // Start the streaming task
        let docker = self.docker.clone();
        let app_state = app_state.clone();
        let active_streams = self.active_streams.clone();
        let container_id = container_id.clone();

        tokio::spawn(async move {
            info!("Starting log stream {} for container {}", stream_id, container_id);

            let options = Some(LogsOptions {
                stdout: true,
                stderr: true,
                follow,
                timestamps: true,
                tail: tail.unwrap_or_else(|| "100".to_string()),
                ..Default::default()
            });

            let mut stream = docker.logs(&container_id, options);
            let mut sequence = 0u64;
            let mut lines_buffer = Vec::new();
            let mut last_send = tokio::time::Instant::now();

            loop {
                tokio::select! {
                    // Check for control commands
                    Some(cmd) = rx.recv() => {
                        match cmd {
                            LogStreamCommand::Stop => {
                                info!("Stopping log stream {} by request", stream_id);
                                break;
                            }
                        }
                    }
                    // Process log output
                    Some(result) = stream.next() => {
                        match result {
                            Ok(log_output) => {
                                let (stream_type, content) = match log_output {
                                    LogOutput::StdOut { message } => {
                                        (OutputStreamType::Stdout, String::from_utf8_lossy(&message).to_string())
                                    }
                                    LogOutput::StdErr { message } => {
                                        (OutputStreamType::Stderr, String::from_utf8_lossy(&message).to_string())
                                    }
                                    LogOutput::StdIn { .. } => continue, // Skip stdin
                                    LogOutput::Console { .. } => continue, // Skip console
                                };

                                // Parse timestamp if present (Docker includes it when timestamps: true)
                                let (timestamp, clean_content) = extract_docker_timestamp(&content);

                                let output_line = OutputLine {
                                    timestamp: timestamp.unwrap_or_else(|| Utc::now()),
                                    stream: stream_type,
                                    content: clean_content,
                                    sequence,
                                };
                                sequence += 1;

                                lines_buffer.push(output_line);

                                // Send buffered lines if we have enough or enough time has passed
                                if lines_buffer.len() >= 10 || last_send.elapsed() > tokio::time::Duration::from_millis(100) {
                                    if !lines_buffer.is_empty() {
                                        broadcast_message(
                                            &app_state,
                                            WebSocketMessage::LogsStreamData(LogsStreamData {
                                                stream_id,
                                                lines: lines_buffer.clone(),
                                            }),
                                        ).await;
                                        lines_buffer.clear();
                                        last_send = tokio::time::Instant::now();
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error reading logs for stream {}: {}", stream_id, e);
                                broadcast_message(
                                    &app_state,
                                    WebSocketMessage::LogsStreamError(LogsStreamError {
                                        stream_id,
                                        error: e.to_string(),
                                    }),
                                ).await;
                                break;
                            }
                        }
                    }
                    // Stream ended
                    else => {
                        info!("Log stream {} ended (no more output)", stream_id);
                        break;
                    }
                }
            }

            // Send any remaining buffered lines
            if !lines_buffer.is_empty() {
                broadcast_message(
                    &app_state,
                    WebSocketMessage::LogsStreamData(LogsStreamData {
                        stream_id,
                        lines: lines_buffer,
                    }),
                ).await;
            }

            // Clean up and send end message
            {
                let mut streams = active_streams.write().await;
                streams.remove(&stream_id);
            }

            broadcast_message(
                &app_state,
                WebSocketMessage::LogsStreamEnded(LogsStreamEnd {
                    stream_id,
                    reason: "Stream ended".to_string(),
                }),
            ).await;

            info!("Log stream {} cleaned up", stream_id);
        });

        Ok(stream_id)
    }

    /// Stop a log stream
    pub async fn stop_stream(&self, stream_id: Uuid) -> Result<(), String> {
        let streams = self.active_streams.read().await;
        if let Some(session) = streams.get(&stream_id) {
            session.sender.send(LogStreamCommand::Stop).await
                .map_err(|_| "Failed to send stop command".to_string())?;
            Ok(())
        } else {
            Err(format!("Stream {} not found", stream_id))
        }
    }

    /// Stop all streams for an app
    pub async fn stop_app_streams(&self, app_name: &str) {
        let streams = self.active_streams.read().await;
        let app_streams: Vec<_> = streams.values()
            .filter(|s| s.app_name == app_name)
            .cloned()
            .collect();
        drop(streams);

        for session in app_streams {
            let _ = session.sender.send(LogStreamCommand::Stop).await;
        }
    }

    /// Get active streams
    pub async fn get_active_streams(&self) -> Vec<LogStreamSession> {
        let streams = self.active_streams.read().await;
        streams.values().cloned().collect()
    }
}

/// Extract Docker timestamp from log line if present
/// Docker format: "2024-01-15T10:30:45.123456789Z message content"
fn extract_docker_timestamp(content: &str) -> (Option<chrono::DateTime<Utc>>, String) {
    // Check if line starts with a timestamp
    if content.len() > 30 && &content[..1] == "2" {
        // Try to find the end of the timestamp (usually a space after 'Z')
        if let Some(pos) = content.find(" ") {
            let timestamp_str = &content[..pos];
            if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(timestamp_str) {
                let clean_content = content[pos + 1..].to_string();
                return (Some(timestamp.with_timezone(&Utc)), clean_content);
            }
        }
    }
    (None, content.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_docker_timestamp() {
        let input = "2024-01-15T10:30:45.123456789Z Hello, world!";
        let (timestamp, content) = extract_docker_timestamp(input);

        assert!(timestamp.is_some());
        assert_eq!(content, "Hello, world!");

        let input_no_timestamp = "Regular log line without timestamp";
        let (timestamp, content) = extract_docker_timestamp(input_no_timestamp);

        assert!(timestamp.is_none());
        assert_eq!(content, "Regular log line without timestamp");
    }
}