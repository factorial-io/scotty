use scotty_core::tasks::task_details::TaskDetails;
use scotty_core::output::OutputLine;
use serde::{Deserialize, Serialize};
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

    // Log streaming messages
    LogsStreamStarted(LogsStreamInfo),
    LogsStreamData(LogsStreamData),
    LogsStreamEnded(LogsStreamEnd),
    LogsStreamError(LogsStreamError),

    // Shell session messages
    ShellSessionCreated(ShellSessionInfo),
    ShellSessionData(ShellSessionData),
    ShellSessionEnded(ShellSessionEnd),
    ShellSessionError(ShellSessionError),
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

/// Information about a created shell session
#[derive(Serialize, Deserialize, Debug)]
pub struct ShellSessionInfo {
    pub session_id: Uuid,
    pub app_name: String,
    pub service_name: String,
    pub container_id: String,
    pub shell_command: String,
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
    Input,   // Data from client to shell
    Output,  // Data from shell to client
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
