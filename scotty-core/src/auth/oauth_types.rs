use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

/// Device flow response from OAuth provider
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeviceFlowResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    /// Optional complete verification URI with embedded user code
    pub verification_uri_complete: Option<String>,
    /// Token expiration time in seconds
    pub expires_in: u64,
    /// Recommended polling interval in seconds
    pub interval: Option<u64>,
}

/// Token response from OAuth provider
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub user_id: String,
    pub user_name: String,
    pub user_email: String,
    /// Optional refresh token
    pub refresh_token: Option<String>,
    /// Optional token expiration time in seconds
    pub expires_in: Option<u64>,
}

/// OAuth error types combining internal errors and RFC 6749 standard codes
#[derive(Debug, Clone, Error, Serialize, Deserialize, ToSchema)]
#[serde(tag = "error", content = "error_description")]
pub enum OAuthError {
    /// OAuth is not configured for this server
    #[error("OAuth not configured")]
    #[serde(rename = "oauth_not_configured")]
    OauthNotConfigured,

    /// Authorization request is pending user approval
    #[error("Authorization pending")]
    #[serde(rename = "authorization_pending")]
    AuthorizationPending,

    /// User denied the authorization request
    #[error("Access denied")]
    #[serde(rename = "access_denied")]
    AccessDenied,

    /// Internal server error occurred
    #[error("Server error: {0}")]
    #[serde(rename = "server_error")]
    ServerError(String),

    /// Invalid request parameters
    #[error("Invalid request: {0}")]
    #[serde(rename = "invalid_request")]
    InvalidRequest(String),

    /// Token or device code has expired
    #[error("Token expired")]
    #[serde(rename = "expired_token")]
    ExpiredToken,

    /// Session has expired
    #[error("Session expired")]
    #[serde(rename = "expired_session")]
    ExpiredSession,

    /// Session not found
    #[error("Session not found")]
    #[serde(rename = "session_not_found")]
    SessionNotFound,

    /// Client is polling too frequently
    #[error("Slow down")]
    #[serde(rename = "slow_down")]
    SlowDown,

    /// Invalid state parameter
    #[error("Invalid state parameter")]
    #[serde(rename = "invalid_state")]
    InvalidState,

    /// OAuth2 library error
    #[error("OAuth2 error: {0}")]
    #[serde(rename = "oauth2_error")]
    OAuth2(String),

    /// HTTP request error
    #[error("HTTP error: {0}")]
    #[serde(rename = "http_error")]
    Http(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    #[serde(rename = "serialization_error")]
    Serialization(String),

    /// URL parse error
    #[error("URL parse error: {0}")]
    #[serde(rename = "url_error")]
    UrlParse(String),
}

impl OAuthError {
    /// Get the OAuth2 RFC-compliant error code
    pub fn code(&self) -> &str {
        match self {
            OAuthError::OauthNotConfigured => "oauth_not_configured",
            OAuthError::AuthorizationPending => "authorization_pending",
            OAuthError::AccessDenied => "access_denied",
            OAuthError::ServerError(_) => "server_error",
            OAuthError::InvalidRequest(_) => "invalid_request",
            OAuthError::ExpiredToken => "expired_token",
            OAuthError::ExpiredSession => "expired_session",
            OAuthError::SessionNotFound => "session_not_found",
            OAuthError::SlowDown => "slow_down",
            OAuthError::InvalidState => "invalid_state",
            OAuthError::OAuth2(_) => "server_error",
            OAuthError::Http(_) => "server_error",
            OAuthError::Serialization(_) => "server_error",
            OAuthError::UrlParse(_) => "invalid_request",
        }
    }
}

impl From<OAuthError> for axum::http::StatusCode {
    fn from(error: OAuthError) -> Self {
        match error {
            OAuthError::OauthNotConfigured => axum::http::StatusCode::NOT_FOUND,
            OAuthError::AuthorizationPending => axum::http::StatusCode::BAD_REQUEST,
            OAuthError::AccessDenied => axum::http::StatusCode::FORBIDDEN,
            OAuthError::ServerError(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            OAuthError::InvalidRequest(_) => axum::http::StatusCode::BAD_REQUEST,
            OAuthError::ExpiredToken => axum::http::StatusCode::UNAUTHORIZED,
            OAuthError::ExpiredSession => axum::http::StatusCode::UNAUTHORIZED,
            OAuthError::SessionNotFound => axum::http::StatusCode::NOT_FOUND,
            OAuthError::SlowDown => axum::http::StatusCode::TOO_MANY_REQUESTS,
            OAuthError::InvalidState => axum::http::StatusCode::BAD_REQUEST,
            OAuthError::OAuth2(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            OAuthError::Http(_) => axum::http::StatusCode::BAD_GATEWAY,
            OAuthError::Serialization(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            OAuthError::UrlParse(_) => axum::http::StatusCode::BAD_REQUEST,
        }
    }
}

// Conversions from common error types
impl From<reqwest::Error> for OAuthError {
    fn from(err: reqwest::Error) -> Self {
        OAuthError::Http(err.to_string())
    }
}

impl From<serde_json::Error> for OAuthError {
    fn from(err: serde_json::Error) -> Self {
        OAuthError::Serialization(err.to_string())
    }
}

impl From<url::ParseError> for OAuthError {
    fn from(err: url::ParseError) -> Self {
        OAuthError::UrlParse(err.to_string())
    }
}

/// OAuth error response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub error_description: Option<String>,
}

/// Query parameters for device token polling
#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
pub struct DeviceTokenQuery {
    pub device_code: String,
}

impl DeviceFlowResponse {
    /// Get the polling interval, defaulting to 5 seconds if not specified
    pub fn polling_interval(&self) -> u64 {
        self.interval.unwrap_or(5)
    }
}

impl From<OAuthError> for ErrorResponse {
    fn from(error: OAuthError) -> Self {
        Self {
            error: error.code().to_string(),
            error_description: Some(error.to_string()),
        }
    }
}

impl ErrorResponse {
    /// Create an error response from OAuthError (convenience method)
    pub fn from(error: OAuthError) -> Self {
        error.into()
    }

    /// Create an error response with custom description
    pub fn with_description(error: OAuthError, description: impl Into<String>) -> Self {
        Self {
            error: error.code().to_string(),
            error_description: Some(description.into()),
        }
    }

    /// Create an error response from a raw string (for compatibility)
    pub fn from_string(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            error_description: None,
        }
    }

    /// Create an error response from a raw string with description (for compatibility)
    pub fn from_string_with_description(
        error: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            error: error.into(),
            error_description: Some(description.into()),
        }
    }

    /// Get the error description, falling back to the error code
    pub fn description(&self) -> &str {
        self.error_description.as_deref().unwrap_or(&self.error)
    }
}
