use crate::output::OutputLine;
use crate::tasks::task_details::TaskDetails;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
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
        stream_id: Uuid,
    },

    // Log streaming response messages (server → client)
    LogsStreamStarted(LogsStreamInfo),
    LogsStreamData(LogsStreamData),
    LogsStreamEnded(LogsStreamEnd),
    LogsStreamError(LogsStreamError),

    // Shell session messages
    ShellSessionCreated(ShellSessionInfo),
    ShellSessionData(ShellSessionData),
    ShellSessionEnded(ShellSessionEnd),
    ShellSessionError(ShellSessionError),

    // Task output streaming messages
    StartTaskOutputStream {
        task_id: Uuid,
        from_beginning: bool, // true = send all history first (default)
    },
    StopTaskOutputStream {
        task_id: Uuid,
    },
    TaskOutputStreamStarted {
        task_id: Uuid,
        total_lines: u64, // Total lines available at start
    },
    TaskOutputData(TaskOutputData),
    TaskOutputStreamEnded {
        task_id: Uuid,
        reason: String, // "completed", "failed", "expired", "deleted"
    },
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

/// Information about a started log stream
#[derive(Serialize, Deserialize, Debug)]
pub struct LogsStreamInfo {
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

/// Information about a created shell session
#[derive(Serialize, Deserialize, Debug)]
pub struct ShellSessionInfo {
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

/// Shell session data (input/output)
#[derive(Serialize, Deserialize, Debug)]
pub struct ShellSessionData {
    pub session_id: Uuid,
    pub data_type: ShellDataType,
    pub data: String,
}

/// Type of shell session data
#[derive(Serialize, Deserialize, Debug)]
pub enum ShellDataType {
    Input,  // Data from client to shell
    Output, // Data from shell to client
}

/// Shell session ended notification
#[derive(Serialize, Deserialize, Debug)]
pub struct ShellSessionEnd {
    pub session_id: Uuid,
    pub exit_code: Option<i32>,
    pub reason: String,
}

/// Shell session error notification
#[derive(Serialize, Deserialize, Debug)]
pub struct ShellSessionError {
    pub session_id: Uuid,
    pub error: String,
}

/// Task output data from a stream
#[derive(Serialize, Deserialize, Debug)]
pub struct TaskOutputData {
    pub task_id: Uuid,
    pub lines: Vec<OutputLine>,
    pub is_historical: bool, // true = catching up, false = live
    pub has_more: bool,      // true if more historical data coming
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
            WebSocketMessage::ShellSessionCreated(info) => {
                write!(
                    f,
                    "Shell session created for {}/{} ({})",
                    info.app_name, info.service_name, info.session_id
                )
            }
            WebSocketMessage::ShellSessionData(data) => {
                write!(
                    f,
                    "Shell session {} data ({:?})",
                    data.session_id, data.data_type
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
