//! Shared types for the file-transfer API.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Default maximum transfer size (1 GiB) for file uploads and downloads.
pub const DEFAULT_MAX_TRANSFER_SIZE: u64 = 1024 * 1024 * 1024;

/// Stable error code for file transfer failures. Stable across releases so
/// CLIs and other clients can branch on it without parsing English text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum FileTransferErrorCode {
    /// The `path` query parameter was empty or not absolute.
    InvalidPath,
    /// The target service exists but has no running container.
    ServiceNotRunning,
    /// The caller does not have the required permission.
    Forbidden,
    /// The app, service, or container path could not be found.
    NotFound,
    /// The configured maximum transfer size was exceeded.
    PayloadTooLarge,
    /// Any other error (Docker daemon failure, unexpected I/O, etc.).
    Internal,
}

/// JSON error body returned by the file-transfer endpoints when the request
/// cannot be served.
#[derive(Debug, Clone, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export)]
pub struct FileTransferError {
    /// Machine-readable error code.
    pub code: FileTransferErrorCode,
    /// Human-readable error message; safe to display to operators.
    pub message: String,
}

impl FileTransferError {
    pub fn new(code: FileTransferErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Query parameters accepted by the file-transfer endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferQuery {
    /// Absolute path inside the target container.
    pub path: String,
}
