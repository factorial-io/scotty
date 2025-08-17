pub mod config;
pub mod device_flow;
pub mod storage;

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<SystemTime>,
    pub user_email: String,
    pub user_name: String,
    pub server_url: String, // Remember which server this token is for
}

#[derive(Debug, Clone)]
pub struct OAuthConfig {
    #[allow(dead_code)]
    pub enabled: bool,
    #[allow(dead_code)]
    pub provider: String,
    pub scotty_server_url: String,
    #[allow(dead_code)]
    pub oidc_issuer_url: String,
    #[allow(dead_code)]
    pub client_id: String,
    #[allow(dead_code)]
    pub device_flow_enabled: bool,
}

#[derive(Debug)]
pub enum AuthMethod {
    OAuth(StoredToken),
    #[allow(dead_code)]
    Bearer(String),
    None,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("OAuth not configured on server")]
    OAuthNotConfigured,
    #[error("Device flow not enabled")]
    DeviceFlowNotEnabled,
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Configuration directory not found")]
    ConfigDirNotFound,
    #[error("Authorization pending")]
    AuthorizationPending,
    #[error("Device flow timed out")]
    Timeout,
    #[error("Server error")]
    ServerError,
    #[allow(dead_code)]
    #[error("Token validation failed")]
    TokenValidationFailed,
    #[allow(dead_code)]
    #[error("No authentication method available")]
    NoAuthMethodAvailable,
    #[error("Invalid server response")]
    InvalidResponse,
}
