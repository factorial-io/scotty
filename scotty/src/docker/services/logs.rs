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

use thiserror::Error;

/// Error types for log streaming operations
#[derive(Error, Debug, Clone, utoipa::ToSchema)]
pub enum LogStreamError {
    #[error("Service '{service}' not found in app '{app}'")]
    ServiceNotFound { service: String, app: String },

    #[error("Service '{service}' has no container ID")]
    NoContainerId { service: String },

    #[error("Stream '{stream_id}' not found")]
    StreamNotFound { stream_id: Uuid },

    #[error("Failed to send command to stream: {reason}")]
    CommandSendFailed { reason: String },

    #[error("Docker operation failed: {operation} - {message}")]
    DockerOperationFailed {
        operation: String,
        message: String,
    },
}

/// Result type alias for log streaming operations
pub type LogStreamResult<T> = Result<T, LogStreamError>;

/// Active log streams tracked by the service
#[derive(Debug, Clone)]
pub struct LogStreamSession {
    pub stream_id: Uuid,
    pub app_name: String,
    pub service_name: String,
    pub container_id: String,
    pub sender: mpsc::Sender<LogStreamCommand>,
}

impl LogStreamSession {
    /// Convert to LogsStreamInfo for WebSocket messages
    pub fn to_info(&self, follow: bool) -> LogsStreamInfo {
        LogsStreamInfo::new(
            self.stream_id,
            self.app_name.clone(),
            self.service_name.clone(),
            follow,
        )
    }

    /// Send a stop command to this session
    pub async fn stop(&self) -> LogStreamResult<()> {
        self.sender.send(LogStreamCommand::Stop).await
            .map_err(|e| LogStreamError::CommandSendFailed {
                reason: e.to_string()
            })
    }
}

/// Commands that can be sent to a log stream task
#[derive(Debug)]
pub enum LogStreamCommand {
    Stop,
}

/// Helper for converting LogOutput to OutputLine
struct LogOutputConverter {
    sequence: u64,
}

impl LogOutputConverter {
    fn new() -> Self {
        Self { sequence: 0 }
    }

    fn convert(&mut self, log_output: LogOutput) -> Option<OutputLine> {
        let (stream_type, content) = match log_output {
            LogOutput::StdOut { message } => {
                (OutputStreamType::Stdout, String::from_utf8_lossy(&message).to_string())
            }
            LogOutput::StdErr { message } => {
                (OutputStreamType::Stderr, String::from_utf8_lossy(&message).to_string())
            }
            LogOutput::StdIn { .. } | LogOutput::Console { .. } => return None,
        };

        let (timestamp, clean_content) = extract_docker_timestamp(&content);
        let output_line = OutputLine {
            timestamp: timestamp.unwrap_or_else(Utc::now),
            stream: stream_type,
            content: clean_content,
            sequence: self.sequence,
        };
        self.sequence += 1;
        Some(output_line)
    }
}

/// Buffer for log lines with automatic flushing
struct LogBuffer {
    lines: Vec<OutputLine>,
    last_send: tokio::time::Instant,
    max_lines: usize,
    max_delay_ms: u64,
}

impl LogBuffer {
    fn new(max_lines: usize, max_delay_ms: u64) -> Self {
        Self {
            lines: Vec::new(),
            last_send: tokio::time::Instant::now(),
            max_lines,
            max_delay_ms,
        }
    }

    fn push(&mut self, line: OutputLine) {
        self.lines.push(line);
    }

    fn should_flush(&self) -> bool {
        self.lines.len() >= self.max_lines
            || self.last_send.elapsed() > tokio::time::Duration::from_millis(self.max_delay_ms)
    }

    fn flush(&mut self) -> Vec<OutputLine> {
        self.last_send = tokio::time::Instant::now();
        std::mem::take(&mut self.lines)
    }

    fn has_data(&self) -> bool {
        !self.lines.is_empty()
    }
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
    ) -> LogStreamResult<Uuid> {
        // Find the container for the service
        let container_id = app_data
            .get_container_id_for_service(service_name)
            .ok_or_else(|| {
                if app_data.find_container_by_service(service_name).is_some() {
                    LogStreamError::NoContainerId {
                        service: service_name.to_string()
                    }
                } else {
                    LogStreamError::ServiceNotFound {
                        service: service_name.to_string(),
                        app: app_data.name.clone()
                    }
                }
            })?;

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
            WebSocketMessage::LogsStreamStarted(LogsStreamInfo::new(
                stream_id,
                app_data.name.clone(),
                service_name.to_string(),
                follow,
            )),
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
            let mut converter = LogOutputConverter::new();
            let mut buffer = LogBuffer::new(10, 100);

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
                                if let Some(output_line) = converter.convert(log_output) {
                                    buffer.push(output_line);

                                    // Send buffered lines if we should flush
                                    if buffer.should_flush() && buffer.has_data() {
                                        broadcast_message(
                                            &app_state,
                                            WebSocketMessage::LogsStreamData(LogsStreamData {
                                                stream_id,
                                                lines: buffer.flush(),
                                            }),
                                        ).await;
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
            if buffer.has_data() {
                broadcast_message(
                    &app_state,
                    WebSocketMessage::LogsStreamData(LogsStreamData {
                        stream_id,
                        lines: buffer.flush(),
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
    pub async fn stop_stream(&self, stream_id: Uuid) -> LogStreamResult<()> {
        let streams = self.active_streams.read().await;
        if let Some(session) = streams.get(&stream_id) {
            session.stop().await
        } else {
            Err(LogStreamError::StreamNotFound { stream_id })
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