use serde::{Deserialize, Serialize};
use std::fmt;
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

/// Standard OAuth2 error codes as defined in RFC 6749
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OAuthErrorCode {
    /// OAuth is not configured for this server
    OauthNotConfigured,
    /// Authorization request is pending user approval
    AuthorizationPending,
    /// User denied the authorization request  
    AccessDenied,
    /// Internal server error occurred
    ServerError,
    /// Invalid request parameters
    InvalidRequest,
    /// Device code has expired
    ExpiredToken,
    /// Invalid session or session not found
    InvalidSession,
    /// Session has expired
    ExpiredSession,
    /// Session not found
    SessionNotFound,
    /// Invalid state parameter
    InvalidState,
}

impl OAuthErrorCode {
    /// Get the standard error description for this error code
    pub fn description(&self) -> &'static str {
        match self {
            OAuthErrorCode::OauthNotConfigured => "OAuth is not configured for this server",
            OAuthErrorCode::AuthorizationPending => "The authorization request is still pending",
            OAuthErrorCode::AccessDenied => "The authorization request was denied",
            OAuthErrorCode::ServerError => "Internal server error occurred",
            OAuthErrorCode::InvalidRequest => "Invalid request parameters",
            OAuthErrorCode::ExpiredToken => "The device code has expired",
            OAuthErrorCode::InvalidSession => "Invalid session or session not found",
            OAuthErrorCode::ExpiredSession => "OAuth session has expired",
            OAuthErrorCode::SessionNotFound => "OAuth session not found or already used",
            OAuthErrorCode::InvalidState => "Invalid state parameter",
        }
    }

    /// Get the OAuth2 error code as a string
    pub fn code(&self) -> &'static str {
        match self {
            OAuthErrorCode::OauthNotConfigured => "oauth_not_configured",
            OAuthErrorCode::AuthorizationPending => "authorization_pending",
            OAuthErrorCode::AccessDenied => "access_denied",
            OAuthErrorCode::ServerError => "server_error",
            OAuthErrorCode::InvalidRequest => "invalid_request",
            OAuthErrorCode::ExpiredToken => "expired_token",
            OAuthErrorCode::InvalidSession => "invalid_session",
            OAuthErrorCode::ExpiredSession => "expired_session",
            OAuthErrorCode::SessionNotFound => "session_not_found",
            OAuthErrorCode::InvalidState => "invalid_state",
        }
    }
}

impl fmt::Display for OAuthErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

impl From<OAuthErrorCode> for String {
    fn from(error_code: OAuthErrorCode) -> Self {
        error_code.to_string()
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

impl ErrorResponse {
    /// Create an error response with standard error code and description
    pub fn new(error_code: OAuthErrorCode) -> Self {
        Self {
            error: error_code.code().to_string(),
            error_description: Some(error_code.description().to_string()),
        }
    }

    /// Create an error response with custom description (overrides standard description)
    pub fn with_description(error_code: OAuthErrorCode, description: impl Into<String>) -> Self {
        Self {
            error: error_code.code().to_string(),
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
