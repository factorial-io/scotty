use bollard::container::LogOutput;
use bollard::query_parameters::LogsOptions;
use bollard::Docker;
use chrono::Utc;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info};
use uuid::Uuid;

use crate::app_state::SharedAppState;
use crate::metrics;
use scotty_core::apps::app_data::AppData;
use scotty_core::websocket::message::WebSocketMessage;
use scotty_types::{
    LogsStreamData, LogsStreamEnd, LogsStreamError, LogsStreamInfo, OutputLine, OutputStreamType,
};

use thiserror::Error;

/// Record metrics when a log stream is started
fn record_stream_started_metrics(active_count: usize) {
    if let Some(m) = metrics::get_metrics() {
        m.log_streams_active.record(active_count as i64, &[]);
        m.log_streams_total.add(1, &[]);
    }
}

/// Record metrics when a log line is received
fn record_log_line_received_metrics() {
    if let Some(m) = metrics::get_metrics() {
        m.log_lines_received.add(1, &[]);
    }
}

/// Record metrics when a log stream encounters an error
fn record_stream_error_metrics() {
    if let Some(m) = metrics::get_metrics() {
        m.log_stream_errors.add(1, &[]);
    }
}

/// Record metrics when a log stream ends
fn record_stream_ended_metrics(active_count: usize, duration_secs: f64) {
    if let Some(m) = metrics::get_metrics() {
        m.log_streams_active.record(active_count as i64, &[]);
        m.log_stream_duration.record(duration_secs, &[]);
    }
}

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
    DockerOperationFailed { operation: String, message: String },
}

/// Result type alias for log streaming operations
pub type LogStreamResult<T> = Result<T, LogStreamError>;

/// Active log streams tracked by the service
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LogStreamSession {
    pub stream_id: Uuid,
    pub app_name: String,
    pub service_name: String,
    pub container_id: String,
    pub client_id: Option<Uuid>, // Track which client owns this stream
    pub sender: mpsc::Sender<LogStreamCommand>,
}

impl LogStreamSession {
    /// Convert to LogsStreamInfo for WebSocket messages
    #[allow(dead_code)]
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
        self.sender.send(LogStreamCommand::Stop).await.map_err(|e| {
            LogStreamError::CommandSendFailed {
                reason: e.to_string(),
            }
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
            LogOutput::StdOut { message } => (
                OutputStreamType::Stdout,
                String::from_utf8_lossy(&message).to_string(),
            ),
            LogOutput::StdErr { message } => (
                OutputStreamType::Stderr,
                String::from_utf8_lossy(&message).to_string(),
            ),
            LogOutput::StdIn { .. } | LogOutput::Console { .. } => return None,
        };

        let (timestamp, clean_content) = extract_docker_timestamp(&content);

        // Skip empty or whitespace-only lines
        if clean_content.trim().is_empty() {
            return None;
        }

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
#[derive(Debug, Clone)]
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
        client_id: Option<Uuid>,
    ) -> LogStreamResult<Uuid> {
        // Find the container for the service
        let container_id = app_data
            .get_container_id_for_service(service_name)
            .ok_or_else(|| {
                if app_data.find_container_by_service(service_name).is_some() {
                    LogStreamError::NoContainerId {
                        service: service_name.to_string(),
                    }
                } else {
                    LogStreamError::ServiceNotFound {
                        service: service_name.to_string(),
                        app: app_data.name.clone(),
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
            client_id,
            sender: tx,
        };

        // Store session
        let active_count = {
            let mut streams = self.active_streams.write().await;
            streams.insert(stream_id, session.clone());
            streams.len()
        };

        record_stream_started_metrics(active_count);

        // Send stream started message to the specific client
        if let Some(client_id) = client_id {
            let _ = app_state
                .messenger
                .send_to_client(
                    client_id,
                    WebSocketMessage::LogsStreamStarted(LogsStreamInfo::new(
                        stream_id,
                        app_data.name.clone(),
                        service_name.to_string(),
                        follow,
                    )),
                )
                .await;
        }

        // Start the streaming task
        let docker = self.docker.clone();
        let app_state = app_state.clone();
        let active_streams = self.active_streams.clone();
        let container_id = container_id.clone();
        let app_name = app_data.name.clone();
        let service_name = service_name.to_string();

        crate::metrics::spawn_instrumented(async move {
            // Track stream duration
            let stream_start = std::time::Instant::now();

            info!(
                "Starting log stream {} for container {} (app: '{}', service: '{}', follow: {})",
                stream_id, container_id, app_name, service_name, follow
            );

            let options = Some(LogsOptions {
                stdout: true,
                stderr: true,
                follow,
                timestamps: true,
                tail: tail.unwrap_or_else(|| "all".to_string()),
                ..Default::default()
            });

            let mut stream = docker.logs(&container_id, options);
            info!(
                "Created Docker log stream for container {} with follow={}",
                container_id, follow
            );

            let mut converter = LogOutputConverter::new();
            // Use consistent buffering for both follow and historical modes (50 lines or 100ms)
            // This provides good performance for both CLI and frontend without causing UI lag
            let mut buffer = LogBuffer::new(50, 100);

            let flush_interval = tokio::time::Duration::from_millis(100);
            let mut flush_timer = tokio::time::interval(flush_interval);
            flush_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            // For non-follow mode, add an idle timeout to detect when no more logs are coming
            let mut last_log_time = tokio::time::Instant::now();
            let idle_timeout = if follow {
                tokio::time::Duration::from_secs(3600) // 1 hour for follow mode (effectively no timeout)
            } else {
                tokio::time::Duration::from_millis(200) // 200ms idle timeout for non-follow
            };

            info!("Starting log stream loop for {} (follow={}, buffer_max_lines={}, buffer_max_delay={}ms)",
                 stream_id, follow, buffer.max_lines, buffer.max_delay_ms);

            let mut _logs_received_count = 0;
            loop {
                tokio::select! {
                    // Check for control commands
                    Some(cmd) = rx.recv() => {
                        match cmd {
                            LogStreamCommand::Stop => {
                                info!("Stopping log stream {} by external request", stream_id);
                                break;
                            }
                        }
                    }
                    // Check if we should flush the buffer based on time
                    _ = flush_timer.tick() => {
                        // Check for idle timeout in non-follow mode
                        if !follow && last_log_time.elapsed() > idle_timeout {
                            info!("Log stream {} idle timeout reached (no logs for {}ms), ending stream", stream_id, idle_timeout.as_millis());
                            break;
                        }

                        if buffer.has_data() && buffer.should_flush() {
                            if let Some(client_id) = client_id {
                                let lines_to_send = buffer.flush();
                                let lines_count = lines_to_send.len();
                                if lines_count > 0 {
                                    if follow {
                                        info!("Timer flush: sending {} buffered log lines for follow stream {}", lines_count, stream_id);
                                    }
                                    let _ = app_state.messenger.send_to_client(
                                        client_id,
                                        WebSocketMessage::LogsStreamData(LogsStreamData {
                                            stream_id,
                                            lines: lines_to_send,
                                        }),
                                    ).await;
                                }
                            }
                        }
                    }
                    // Process log output
                    Some(result) = stream.next() => {
                        _logs_received_count += 1;
                        match result {
                            Ok(log_output) => {
                                if let Some(output_line) = converter.convert(log_output) {
                                    last_log_time = tokio::time::Instant::now(); // Reset idle timer
                                    buffer.push(output_line);

                                    record_log_line_received_metrics();

                                    // Send buffered lines if we should flush
                                    if buffer.should_flush() && buffer.has_data() {
                                        if let Some(client_id) = client_id {
                                            let lines_to_send = buffer.flush();
                                            let _lines_count = lines_to_send.len();
                                            let _ = app_state.messenger.send_to_client(
                                                client_id,
                                                WebSocketMessage::LogsStreamData(LogsStreamData {
                                                    stream_id,
                                                    lines: lines_to_send,
                                                }),
                                            ).await;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error reading logs for stream {}: {}", stream_id, e);

                                record_stream_error_metrics();

                                if let Some(client_id) = client_id {
                                    let _ = app_state.messenger.send_to_client(
                                        client_id,
                                        WebSocketMessage::LogsStreamError(LogsStreamError {
                                            stream_id,
                                            error: e.to_string(),
                                        }),
                                    ).await;
                                }
                                break;
                            }
                        }
                    }
                    // Stream ended
                    else => {
                        info!("Log stream {} ended naturally (stream.next() returned None, no more output from container, follow={})", stream_id, follow);
                        break;
                    }
                }
            }

            // Send any remaining buffered lines
            if buffer.has_data() {
                if let Some(client_id) = client_id {
                    let _ = app_state
                        .messenger
                        .send_to_client(
                            client_id,
                            WebSocketMessage::LogsStreamData(LogsStreamData {
                                stream_id,
                                lines: buffer.flush(),
                            }),
                        )
                        .await;
                }
            }

            // Clean up and send end message
            let active_count = {
                let mut streams = active_streams.write().await;
                streams.remove(&stream_id);
                streams.len()
            };

            let duration_secs = stream_start.elapsed().as_secs_f64();
            record_stream_ended_metrics(active_count, duration_secs);

            if let Some(client_id) = client_id {
                info!(
                    "Sending LogsStreamEnded message for stream {} to client {}",
                    stream_id, client_id
                );
                let _ = app_state
                    .messenger
                    .send_to_client(
                        client_id,
                        WebSocketMessage::LogsStreamEnded(LogsStreamEnd {
                            stream_id,
                            reason: if follow {
                                "Stream stopped".to_string()
                            } else {
                                "All logs delivered".to_string()
                            },
                        }),
                    )
                    .await;
                info!("LogsStreamEnded message sent for stream {}", stream_id);
            }

            info!(
                "Log stream {} cleaned up and removed from active streams",
                stream_id
            );
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
    #[allow(dead_code)]
    pub async fn stop_app_streams(&self, app_name: &str) {
        let streams = self.active_streams.read().await;
        let app_streams: Vec<_> = streams
            .values()
            .filter(|s| s.app_name == app_name)
            .cloned()
            .collect();
        drop(streams);

        for session in app_streams {
            let _ = session.sender.send(LogStreamCommand::Stop).await;
        }
    }

    /// Stop all streams for a specific client
    pub async fn stop_client_streams(&self, client_id: Uuid) -> Vec<Uuid> {
        let streams = self.active_streams.read().await;
        let client_streams: Vec<_> = streams
            .values()
            .filter(|s| s.client_id == Some(client_id))
            .cloned()
            .collect();
        drop(streams);

        let mut stopped_stream_ids = Vec::new();
        for session in client_streams {
            if session.sender.send(LogStreamCommand::Stop).await.is_ok() {
                stopped_stream_ids.push(session.stream_id);
            }
        }

        stopped_stream_ids
    }

    /// Get active streams
    #[allow(dead_code)]
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
