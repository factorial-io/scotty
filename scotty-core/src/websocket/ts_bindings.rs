// Spike: Testing ts-rs for WebSocket message type generation
// This module generates TypeScript bindings for WebSocket messages

#[cfg(feature = "ts-rs")]
use ts_rs::TS;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

// Re-export with ts-rs derive
#[cfg(feature = "ts-rs")]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/generated/")]
pub enum OutputStreamType {
    Stdout,
    Stderr,
}

#[cfg(feature = "ts-rs")]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/generated/")]
pub struct OutputLine {
    #[ts(type = "string")]  // DateTime gets converted to string
    pub timestamp: DateTime<Utc>,
    pub stream: OutputStreamType,
    pub content: String,
    pub sequence: u64,
}

#[cfg(feature = "ts-rs")]
#[derive(Serialize, Deserialize, Debug, TS)]
#[ts(export, export_to = "../frontend/src/generated/")]
pub enum ShellDataType {
    Input,
    Output,
}

#[cfg(feature = "ts-rs")]
#[derive(Serialize, Deserialize, Debug, TS)]
#[ts(export, export_to = "../frontend/src/generated/")]
pub struct TaskOutputData {
    #[ts(type = "string")]  // UUID gets converted to string
    pub task_id: Uuid,
    pub lines: Vec<OutputLine>,
    pub is_historical: bool,
    pub has_more: bool,
}

#[cfg(feature = "ts-rs")]
#[derive(Serialize, Deserialize, Debug, TS)]
#[ts(export, export_to = "../frontend/src/generated/")]
pub struct LogStreamRequest {
    pub app_name: String,
    pub service_name: String,
    pub follow: bool,
    pub lines: Option<u32>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub timestamps: bool,
}

#[cfg(feature = "ts-rs")]
#[derive(Serialize, Deserialize, Debug, TS)]
#[ts(export, export_to = "../frontend/src/generated/")]
pub struct LogsStreamInfo {
    #[ts(type = "string")]
    pub stream_id: Uuid,
    pub app_name: String,
    pub service_name: String,
    pub follow: bool,
}

#[cfg(feature = "ts-rs")]
#[derive(Serialize, Deserialize, Debug, TS)]
#[ts(export, export_to = "../frontend/src/generated/")]
pub struct LogsStreamData {
    #[ts(type = "string")]
    pub stream_id: Uuid,
    pub lines: Vec<OutputLine>,
}

#[cfg(feature = "ts-rs")]
#[derive(Serialize, Deserialize, Debug, TS)]
#[ts(export, export_to = "../frontend/src/generated/")]
pub struct LogsStreamEnd {
    #[ts(type = "string")]
    pub stream_id: Uuid,
    pub reason: String,
}

#[cfg(feature = "ts-rs")]
#[derive(Serialize, Deserialize, Debug, TS)]
#[ts(export, export_to = "../frontend/src/generated/")]
pub struct LogsStreamError {
    #[ts(type = "string")]
    pub stream_id: Uuid,
    pub error: String,
}

#[cfg(feature = "ts-rs")]
#[derive(Serialize, Deserialize, Debug, TS)]
#[ts(export, export_to = "../frontend/src/generated/")]
pub struct ShellSessionInfo {
    #[ts(type = "string")]
    pub session_id: Uuid,
    pub app_name: String,
    pub service_name: String,
    pub container_id: String,
    pub shell_command: String,
}

#[cfg(feature = "ts-rs")]
#[derive(Serialize, Deserialize, Debug, TS)]
#[ts(export, export_to = "../frontend/src/generated/")]
pub struct ShellSessionData {
    #[ts(type = "string")]
    pub session_id: Uuid,
    pub data_type: ShellDataType,
    pub data: String,
}

#[cfg(feature = "ts-rs")]
#[derive(Serialize, Deserialize, Debug, TS)]
#[ts(export, export_to = "../frontend/src/generated/")]
pub struct ShellSessionEnd {
    #[ts(type = "string")]
    pub session_id: Uuid,
    pub exit_code: Option<i32>,
    pub reason: String,
}

#[cfg(feature = "ts-rs")]
#[derive(Serialize, Deserialize, Debug, TS)]
#[ts(export, export_to = "../frontend/src/generated/")]
pub struct ShellSessionError {
    #[ts(type = "string")]
    pub session_id: Uuid,
    pub error: String,
}

// Main WebSocket message enum
// Note: This is complex due to TaskDetails dependency, we might need to handle that separately
#[cfg(feature = "ts-rs")]
#[derive(Serialize, Deserialize, Debug, TS)]
#[ts(export, export_to = "../frontend/src/generated/")]
pub enum WebSocketMessage {
    Ping,
    Pong,
    AppListUpdated,
    AppInfoUpdated(String),
    TaskListUpdated,
    // TaskInfoUpdated would need TaskDetails to also implement TS
    Error(String),

    // Authentication
    Authenticate { token: String },
    AuthenticationSuccess,
    AuthenticationFailed { reason: String },

    // Log streaming
    StartLogStream(LogStreamRequest),
    StopLogStream { #[ts(type = "string")] stream_id: Uuid },
    LogsStreamStarted(LogsStreamInfo),
    LogsStreamData(LogsStreamData),
    LogsStreamEnded(LogsStreamEnd),
    LogsStreamError(LogsStreamError),

    // Shell sessions
    ShellSessionCreated(ShellSessionInfo),
    ShellSessionData(ShellSessionData),
    ShellSessionEnded(ShellSessionEnd),
    ShellSessionError(ShellSessionError),

    // Task output streaming
    StartTaskOutputStream {
        #[ts(type = "string")]
        task_id: Uuid,
        from_beginning: bool,
    },
    StopTaskOutputStream {
        #[ts(type = "string")]
        task_id: Uuid,
    },
    TaskOutputStreamStarted {
        #[ts(type = "string")]
        task_id: Uuid,
        total_lines: u64,
    },
    TaskOutputData(TaskOutputData),
    TaskOutputStreamEnded {
        #[ts(type = "string")]
        task_id: Uuid,
        reason: String,
    },
}

#[cfg(test)]
#[cfg(feature = "ts-rs")]
mod tests {
    use super::*;

    #[test]
    fn test_ts_generation() {
        // This test will generate the TypeScript files when run with ts-rs feature
        // Run with: cargo test --features ts-rs -p scotty-core ts_bindings::tests::test_ts_generation

        // The actual generation happens via the #[ts(export)] attributes
        // This test just verifies the types compile correctly

        let _msg = WebSocketMessage::Ping;
        let _output = OutputStreamType::Stdout;
    }
}