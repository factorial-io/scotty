/*!
 * Minimal types-only crate for TypeScript generation
 *
 * This crate contains only the essential types needed for frontend TypeScript
 * generation, without any of the heavy dependencies required by the full
 * scotty-core library.
 */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;
use uuid::Uuid;

// Re-export core types for easier access
pub use chrono;
pub use serde;
pub use ts_rs;
pub use uuid;

// Output types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[ts(export)]
pub enum OutputStreamType {
    Stdout,
    Stderr,
    /// Status messages (e.g., "Starting...", "Running...", "Completed")
    Status,
    /// Error status messages (e.g., "Failed", "Error occurred")
    StatusError,
    /// Progress updates (e.g., "Step 2/5: Installing dependencies...")
    Progress,
    /// Information/debug messages from the system
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[ts(export)]
pub struct OutputLine {
    /// Timestamp when the line was received
    #[ts(type = "string")]
    pub timestamp: DateTime<Utc>,
    /// Type of stream (stdout or stderr)
    pub stream: OutputStreamType,
    /// The actual content of the line
    pub content: String,
    /// Sequence number for ordering guarantee
    pub sequence: u64,
}

impl OutputLine {
    pub fn new(stream: OutputStreamType, content: String, sequence: u64) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            stream,
            content,
            sequence,
        }
    }

    pub fn stdout(content: String, sequence: u64) -> Self {
        Self::new(OutputStreamType::Stdout, content, sequence)
    }

    pub fn stderr(content: String, sequence: u64) -> Self {
        Self::new(OutputStreamType::Stderr, content, sequence)
    }
}

impl std::fmt::Display for OutputStreamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputStreamType::Stdout => write!(f, "stdout"),
            OutputStreamType::Stderr => write!(f, "stderr"),
            OutputStreamType::Status => write!(f, "status"),
            OutputStreamType::StatusError => write!(f, "status-error"),
            OutputStreamType::Progress => write!(f, "progress"),
            OutputStreamType::Info => write!(f, "info"),
        }
    }
}

// Output collection configuration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[ts(export)]
pub struct OutputLimits {
    /// Maximum number of lines to keep in memory
    pub max_lines: usize,
    /// Maximum length of a single line (characters)
    pub max_line_length: usize,
}

impl Default for OutputLimits {
    fn default() -> Self {
        Self {
            max_lines: 10000,
            max_line_length: 4096,
        }
    }
}

// Unified output collection for tasks
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[ts(export)]
pub struct TaskOutput {
    /// Collected output lines in chronological order
    pub lines: Vec<OutputLine>,
    /// Configuration limits (skipped in serialization but exported to TS)
    pub limits: OutputLimits,
    /// Total number of lines that have been processed (including evicted ones)
    pub total_lines_processed: u64,
    /// Current sequence number for new lines (skipped in serialization but exported to TS)
    pub current_sequence: u64,
}

impl Default for TaskOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskOutput {
    pub fn new() -> Self {
        Self::with_limits(OutputLimits::default())
    }

    pub fn with_limits(limits: OutputLimits) -> Self {
        Self {
            lines: Vec::new(),
            limits,
            total_lines_processed: 0,
            current_sequence: 0,
        }
    }

    /// Create TaskOutput with custom limits
    pub fn new_with_limits(max_lines: usize, max_line_length: usize) -> Self {
        let limits = OutputLimits {
            max_lines,
            max_line_length,
        };
        Self::with_limits(limits)
    }

    /// Add stdout line
    pub fn add_stdout(&mut self, content: String) {
        self.add_line(OutputStreamType::Stdout, content);
    }

    /// Add stderr line
    pub fn add_stderr(&mut self, content: String) {
        self.add_line(OutputStreamType::Stderr, content);
    }

    /// Get total number of lines currently in memory
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Check if any lines have been evicted due to limits
    pub fn has_truncated_history(&self) -> bool {
        self.total_lines_processed > self.lines.len() as u64
    }

    /// Clear all output lines
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// Add a new output line
    pub fn add_line(&mut self, stream: OutputStreamType, content: String) {
        let line = OutputLine::new(stream, content, self.current_sequence);
        self.current_sequence += 1;
        self.total_lines_processed += 1;

        // Truncate line if too long (UTF-8 safe)
        let mut line = line;
        if line.content.len() > self.limits.max_line_length {
            let truncation_marker = "...[truncated]";
            let available_chars = self
                .limits
                .max_line_length
                .saturating_sub(truncation_marker.len());

            // Use char_indices to find UTF-8 safe truncation point
            let truncate_pos = line
                .content
                .char_indices()
                .nth(available_chars)
                .map(|(pos, _)| pos)
                .unwrap_or(line.content.len());
            line.content = format!("{}...[truncated]", &line.content[..truncate_pos]);
        }

        self.lines.push(line);

        // Remove oldest lines if we exceed the limit
        while self.lines.len() > self.limits.max_lines {
            self.lines.remove(0);
        }
    }

    /// Get recent lines with optional limit
    pub fn get_recent_lines(&self, limit: Option<usize>) -> Vec<OutputLine> {
        match limit {
            Some(n) => self.lines.iter().rev().take(n).rev().cloned().collect(),
            None => self.lines.to_vec(),
        }
    }

    /// Get lines filtered by stream type
    pub fn get_lines_by_stream(&self, stream_type: OutputStreamType) -> Vec<OutputLine> {
        self.lines
            .iter()
            .filter(|line| line.stream == stream_type)
            .cloned()
            .collect()
    }
}

// Task types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::ToResponse))]
#[ts(export)]
pub enum State {
    Running,
    Finished,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToResponse, utoipa::ToSchema))]
#[ts(export)]
pub struct TaskDetails {
    #[ts(type = "string")]
    pub id: Uuid,
    pub command: String,
    pub state: State,
    #[ts(type = "string")]
    pub start_time: DateTime<Utc>,
    #[ts(type = "string | null")]
    pub finish_time: Option<DateTime<Utc>>,
    pub last_exit_code: Option<i32>,
    pub app_name: Option<String>,
    pub output_collection_active: bool,
    /// Embedded task output (not serialized by default)
    #[serde(skip)]
    #[ts(skip)]
    pub output: TaskOutput,
}

impl Default for TaskDetails {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            command: "".to_string(),
            state: State::Running,
            start_time: chrono::Utc::now(),
            finish_time: None,
            last_exit_code: None,
            app_name: None,
            output_collection_active: true,
            output: TaskOutput::default(),
        }
    }
}

impl TaskDetails {
    pub fn new(command: String, app_name: Option<String>) -> Self {
        let id = uuid::Uuid::new_v4();
        Self {
            id,
            command,
            state: State::Running,
            start_time: chrono::Utc::now(),
            finish_time: None,
            last_exit_code: None,
            app_name,
            output_collection_active: true,
            output: TaskOutput::default(),
        }
    }

    /// Create TaskDetails with custom output limits
    pub fn new_with_output_limits(
        command: String,
        app_name: Option<String>,
        max_lines: usize,
        max_line_length: usize,
    ) -> Self {
        let id = uuid::Uuid::new_v4();
        Self {
            id,
            command,
            state: State::Running,
            start_time: chrono::Utc::now(),
            finish_time: None,
            last_exit_code: None,
            app_name,
            output_collection_active: true,
            output: TaskOutput::new_with_limits(max_lines, max_line_length),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TaskOutputData {
    #[ts(type = "string")]
    pub task_id: Uuid,
    pub lines: Vec<OutputLine>,
    pub is_historical: bool, // true = catching up, false = live
    pub has_more: bool,      // true if more historical data coming
}

// Log streaming types
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LogStreamRequest {
    pub app_name: String,
    pub service_name: String,
    pub follow: bool,
    pub lines: Option<u32>, // Number of lines for historical logs (default 100)
    pub since: Option<String>, // Time filter: "1h", "30m", or ISO timestamp
    pub until: Option<String>, // End time filter: ISO timestamp
    pub timestamps: bool,   // Include timestamps in output (default true)
}

impl LogStreamRequest {
    pub fn new(app_name: String, service_name: String, follow: bool) -> Self {
        Self {
            app_name,
            service_name,
            follow,
            lines: Some(100),
            since: None,
            until: None,
            timestamps: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LogsStreamInfo {
    #[ts(type = "string")]
    pub stream_id: Uuid,
    pub app_name: String,
    pub service_name: String,
    pub follow: bool,
}

impl LogsStreamInfo {
    pub fn new(stream_id: Uuid, app_name: String, service_name: String, follow: bool) -> Self {
        Self {
            stream_id,
            app_name,
            service_name,
            follow,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LogsStreamData {
    #[ts(type = "string")]
    pub stream_id: Uuid,
    pub lines: Vec<OutputLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LogsStreamEnd {
    #[ts(type = "string")]
    pub stream_id: Uuid,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LogsStreamError {
    #[ts(type = "string")]
    pub stream_id: Uuid,
    pub error: String,
}

impl std::fmt::Display for LogsStreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stream {}: {}", self.stream_id, self.error)
    }
}

// Shell session types
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ShellSessionRequest {
    pub app_name: String,
    pub service_name: String,
    pub shell_command: Option<String>, // Optional, uses default shell if None
}

impl ShellSessionRequest {
    pub fn new(app_name: String, service_name: String) -> Self {
        Self {
            app_name,
            service_name,
            shell_command: None,
        }
    }

    pub fn with_command(app_name: String, service_name: String, shell_command: String) -> Self {
        Self {
            app_name,
            service_name,
            shell_command: Some(shell_command),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ShellDataType {
    Input,  // Data from client to shell
    Output, // Data from shell to client
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ShellSessionInfo {
    #[ts(type = "string")]
    pub session_id: Uuid,
    pub app_name: String,
    pub service_name: String,
    pub container_id: String,
    pub shell_command: String,
}

impl ShellSessionInfo {
    pub fn new(
        session_id: Uuid,
        app_name: String,
        service_name: String,
        container_id: String,
        shell_command: String,
    ) -> Self {
        Self {
            session_id,
            app_name,
            service_name,
            container_id,
            shell_command,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ShellSessionData {
    #[ts(type = "string")]
    pub session_id: Uuid,
    pub data_type: ShellDataType,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ShellSessionEnd {
    #[ts(type = "string")]
    pub session_id: Uuid,
    pub exit_code: Option<i32>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ShellSessionError {
    #[ts(type = "string")]
    pub session_id: Uuid,
    pub error: String,
}

// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, tag = "type", content = "data")]
#[serde(tag = "type", content = "data")]
pub enum WebSocketMessage {
    Ping,
    Pong,
    AppListUpdated,
    AppInfoUpdated(String),
    TaskListUpdated,
    TaskInfoUpdated(TaskDetails),
    Error(String),

    // Authentication messages
    Authenticate {
        token: String,
    },
    AuthenticationSuccess,
    AuthenticationFailed {
        reason: String,
    },

    // Log streaming request messages (client → server)
    StartLogStream(LogStreamRequest),
    StopLogStream {
        #[ts(type = "string")]
        stream_id: Uuid,
    },

    // Log streaming response messages (server → client)
    LogsStreamStarted(LogsStreamInfo),
    LogsStreamData(LogsStreamData),
    LogsStreamEnded(LogsStreamEnd),
    LogsStreamError(LogsStreamError),

    // Shell session request messages (client → server)
    CreateShellSession(ShellSessionRequest),
    ResizeShellTty {
        #[ts(type = "string")]
        session_id: Uuid,
        width: u16,
        height: u16,
    },
    TerminateShellSession {
        #[ts(type = "string")]
        session_id: Uuid,
    },

    // Shell session response messages (server → client)
    ShellSessionCreated(ShellSessionInfo),
    ShellSessionData(ShellSessionData),
    ShellSessionEnded(ShellSessionEnd),
    ShellSessionError(ShellSessionError),

    // Task output streaming messages
    StartTaskOutputStream {
        #[ts(type = "string")]
        task_id: Uuid,
        from_beginning: bool, // true = send all history first (default)
    },
    StopTaskOutputStream {
        #[ts(type = "string")]
        task_id: Uuid,
    },
    TaskOutputStreamStarted {
        #[ts(type = "string")]
        task_id: Uuid,
        total_lines: u64, // Total lines available at start
    },
    TaskOutputData(TaskOutputData),
    TaskOutputStreamEnded {
        #[ts(type = "string")]
        task_id: Uuid,
        reason: String, // "completed", "failed", "expired", "deleted"
    },
}

impl fmt::Display for WebSocketMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebSocketMessage::Ping => write!(f, "Ping"),
            WebSocketMessage::Pong => write!(f, "Pong"),
            WebSocketMessage::AppListUpdated => write!(f, "App list updated"),
            WebSocketMessage::AppInfoUpdated(app_name) => {
                write!(f, "App '{}' info updated", app_name)
            }
            WebSocketMessage::TaskListUpdated => write!(f, "Task list updated"),
            WebSocketMessage::TaskInfoUpdated(task) => {
                write!(f, "Task '{}' info updated", task.id)
            }
            WebSocketMessage::Error(error) => write!(f, "Error: {}", error),
            WebSocketMessage::Authenticate { token: _ } => {
                write!(f, "Authentication request")
            }
            WebSocketMessage::AuthenticationSuccess => {
                write!(f, "Authentication successful")
            }
            WebSocketMessage::AuthenticationFailed { reason } => {
                write!(f, "Authentication failed: {}", reason)
            }
            WebSocketMessage::StartLogStream(request) => {
                write!(
                    f,
                    "Start log stream for {}/{}",
                    request.app_name, request.service_name
                )
            }
            WebSocketMessage::StopLogStream { stream_id } => {
                write!(f, "Stop log stream {}", stream_id)
            }
            WebSocketMessage::LogsStreamStarted(info) => {
                write!(
                    f,
                    "Log stream started for {}/{} ({})",
                    info.app_name, info.service_name, info.stream_id
                )
            }
            WebSocketMessage::LogsStreamData(data) => {
                write!(
                    f,
                    "Log data for stream {} ({} lines)",
                    data.stream_id,
                    data.lines.len()
                )
            }
            WebSocketMessage::LogsStreamEnded(end) => {
                write!(f, "Log stream {} ended: {}", end.stream_id, end.reason)
            }
            WebSocketMessage::LogsStreamError(error) => {
                write!(f, "Log stream {} error: {}", error.stream_id, error.error)
            }
            WebSocketMessage::CreateShellSession(request) => {
                write!(
                    f,
                    "Create shell session for {}/{}",
                    request.app_name, request.service_name
                )
            }
            WebSocketMessage::ResizeShellTty {
                session_id,
                width,
                height,
            } => {
                write!(f, "Resize shell TTY {} to {}x{}", session_id, width, height)
            }
            WebSocketMessage::TerminateShellSession { session_id } => {
                write!(f, "Terminate shell session {}", session_id)
            }
            WebSocketMessage::ShellSessionCreated(info) => {
                write!(
                    f,
                    "Shell session created for {}/{} ({})",
                    info.app_name, info.service_name, info.session_id
                )
            }
            WebSocketMessage::ShellSessionData(data) => {
                // Trim data payload to first 50 chars for readability
                let data_preview = if data.data.len() > 50 {
                    format!("{}... ({} bytes)", &data.data[..50], data.data.len())
                } else {
                    format!("{} ({} bytes)", &data.data, data.data.len())
                };
                write!(
                    f,
                    "Shell session {} data ({:?}): {}",
                    data.session_id, data.data_type, data_preview
                )
            }
            WebSocketMessage::ShellSessionEnded(end) => {
                write!(f, "Shell session {} ended: {}", end.session_id, end.reason)
            }
            WebSocketMessage::ShellSessionError(error) => {
                write!(
                    f,
                    "Shell session {} error: {}",
                    error.session_id, error.error
                )
            }
            WebSocketMessage::StartTaskOutputStream {
                task_id,
                from_beginning,
            } => {
                write!(
                    f,
                    "Start task output stream for {} (from_beginning: {})",
                    task_id, from_beginning
                )
            }
            WebSocketMessage::StopTaskOutputStream { task_id } => {
                write!(f, "Stop task output stream for {}", task_id)
            }
            WebSocketMessage::TaskOutputStreamStarted {
                task_id,
                total_lines,
            } => {
                write!(
                    f,
                    "Task output stream started for {} ({} total lines)",
                    task_id, total_lines
                )
            }
            WebSocketMessage::TaskOutputData(data) => {
                let status = if data.is_historical {
                    "historical"
                } else {
                    "live"
                };
                write!(
                    f,
                    "Task {} output data ({} lines, {})",
                    data.task_id,
                    data.lines.len(),
                    status
                )
            }
            WebSocketMessage::TaskOutputStreamEnded { task_id, reason } => {
                write!(f, "Task {} output stream ended: {}", task_id, reason)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_output_line_length_limits() {
        let mut output = TaskOutput::with_limits(OutputLimits {
            max_lines: 10,
            max_line_length: 20,
        });

        let long_line = "a".repeat(50);
        output.add_stdout(long_line);

        let stored_line = output.lines.last().unwrap();
        assert!(stored_line.content.len() <= 20);
        assert!(stored_line.content.ends_with("...[truncated]"));
        println!(
            "Stored line: '{}' (length: {})",
            stored_line.content,
            stored_line.content.len()
        );
    }

    #[test]
    fn test_task_output_utf8_safety() {
        let mut output = TaskOutput::with_limits(OutputLimits {
            max_lines: 10,
            max_line_length: 30,
        });

        // Test with emoji (4-byte UTF-8 characters)
        let emoji_line = "Hello 😀😃😄😁😆😅🤣😂".to_string();
        output.add_stdout(emoji_line);

        let stored_line = output.lines.last().unwrap();
        assert!(stored_line.content.ends_with("...[truncated]"));
        // Verify string is valid UTF-8 (this would panic if slicing was done incorrectly)
        assert!(stored_line.content.is_ascii() || stored_line.content.chars().count() > 0);

        // Test with CJK characters (3-byte UTF-8)
        let cjk_line = "你好世界這是一個很長的中文測試字串".to_string();
        output.add_stdout(cjk_line);

        let stored_line = output.lines.last().unwrap();
        assert!(stored_line.content.ends_with("...[truncated]"));
        assert!(stored_line.content.chars().count() > 0);

        // Test mixed ASCII and multi-byte
        let mixed_line = "Start 😀 Middle 你好 End ".repeat(5);
        output.add_stdout(mixed_line);

        let stored_line = output.lines.last().unwrap();
        assert!(stored_line.content.ends_with("...[truncated]"));
        assert!(stored_line.content.chars().count() > 0);
    }

    // Shell-related type tests
    #[test]
    fn test_shell_session_request_serialization() {
        let req = ShellSessionRequest {
            app_name: "test-app".to_string(),
            service_name: "web".to_string(),
            shell_command: Some("/bin/bash".to_string()),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("test-app"));
        assert!(json.contains("web"));
        assert!(json.contains("/bin/bash"));

        // Deserialize back
        let deserialized: ShellSessionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.app_name, "test-app");
        assert_eq!(deserialized.service_name, "web");
        assert_eq!(deserialized.shell_command, Some("/bin/bash".to_string()));
    }

    #[test]
    fn test_shell_session_request_constructors() {
        // Test new() constructor (no command)
        let req1 = ShellSessionRequest::new("app1".to_string(), "svc1".to_string());
        assert_eq!(req1.app_name, "app1");
        assert_eq!(req1.service_name, "svc1");
        assert_eq!(req1.shell_command, None);

        // Test with_command() constructor
        let req2 = ShellSessionRequest::with_command(
            "app2".to_string(),
            "svc2".to_string(),
            "/bin/sh".to_string(),
        );
        assert_eq!(req2.app_name, "app2");
        assert_eq!(req2.service_name, "svc2");
        assert_eq!(req2.shell_command, Some("/bin/sh".to_string()));
    }

    #[test]
    fn test_shell_data_type_variants() {
        // Test Input variant
        let session_id = Uuid::new_v4();
        let input = ShellSessionData {
            session_id,
            data_type: ShellDataType::Input,
            data: "test command\n".to_string(),
        };

        assert!(matches!(input.data_type, ShellDataType::Input));
        assert_eq!(input.data, "test command\n");

        // Test Output variant
        let output = ShellSessionData {
            session_id,
            data_type: ShellDataType::Output,
            data: "command result\n".to_string(),
        };

        assert!(matches!(output.data_type, ShellDataType::Output));
        assert_eq!(output.data, "command result\n");
    }

    #[test]
    fn test_shell_session_data_serialization() {
        let session_id = Uuid::new_v4();
        let data = ShellSessionData {
            session_id,
            data_type: ShellDataType::Input,
            data: "echo hello".to_string(),
        };

        // Serialize
        let json = serde_json::to_string(&data).unwrap();

        // Deserialize
        let deserialized: ShellSessionData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.session_id, session_id);
        assert!(matches!(deserialized.data_type, ShellDataType::Input));
        assert_eq!(deserialized.data, "echo hello");
    }

    #[test]
    fn test_shell_session_end_with_exit_codes() {
        let session_id = Uuid::new_v4();

        // Test with successful exit (code 0)
        let end_success = ShellSessionEnd {
            session_id,
            exit_code: Some(0),
            reason: "Completed successfully".to_string(),
        };
        assert_eq!(end_success.exit_code, Some(0));

        // Test with error exit (code 1)
        let end_error = ShellSessionEnd {
            session_id,
            exit_code: Some(1),
            reason: "Command failed".to_string(),
        };
        assert_eq!(end_error.exit_code, Some(1));

        // Test with signal termination (code 130 = Ctrl+C)
        let end_signal = ShellSessionEnd {
            session_id,
            exit_code: Some(130),
            reason: "Interrupted".to_string(),
        };
        assert_eq!(end_signal.exit_code, Some(130));

        // Test with no exit code (timeout/error)
        let end_no_code = ShellSessionEnd {
            session_id,
            exit_code: None,
            reason: "Session timeout".to_string(),
        };
        assert_eq!(end_no_code.exit_code, None);
    }

    #[test]
    fn test_shell_session_info_construction() {
        let session_id = Uuid::new_v4();
        let info = ShellSessionInfo::new(
            session_id,
            "my-app".to_string(),
            "web".to_string(),
            "container-123".to_string(),
            "/bin/sh".to_string(),
        );

        assert_eq!(info.session_id, session_id);
        assert_eq!(info.app_name, "my-app");
        assert_eq!(info.service_name, "web");
        assert_eq!(info.container_id, "container-123");
        assert_eq!(info.shell_command, "/bin/sh");
    }

    #[test]
    fn test_shell_session_error_structure() {
        let session_id = Uuid::new_v4();
        let error = ShellSessionError {
            session_id,
            error: "Container not found".to_string(),
        };

        assert_eq!(error.session_id, session_id);
        assert_eq!(error.error, "Container not found");

        // Test serialization
        let json = serde_json::to_string(&error).unwrap();
        let deserialized: ShellSessionError = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.session_id, session_id);
        assert_eq!(deserialized.error, "Container not found");
    }
}
