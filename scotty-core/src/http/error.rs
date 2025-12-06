use reqwest::StatusCode;

/// HTTP client error types that preserve status code information
#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    /// HTTP error response with status code and message
    #[error("HTTP {status}: {message}")]
    Http { status: u16, message: String },

    /// Network-level error (connection, DNS, etc.)
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Request timeout
    #[error("Request timeout")]
    Timeout,

    /// Failed to parse response body
    #[error("Failed to parse response: {0}")]
    ParseError(String),
}

impl HttpError {
    /// Create an HTTP error from status code and message
    pub fn http(status: u16, message: impl Into<String>) -> Self {
        Self::Http {
            status,
            message: message.into(),
        }
    }

    /// Create an HTTP error from a StatusCode and message
    pub fn from_status(status: StatusCode, message: impl Into<String>) -> Self {
        Self::Http {
            status: status.as_u16(),
            message: message.into(),
        }
    }

    /// Get the HTTP status code if this is an HTTP error
    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::Http { status, .. } => Some(*status),
            Self::Network(e) => e.status().map(|s| s.as_u16()),
            _ => None,
        }
    }

    /// Check if this is a client error (4xx)
    pub fn is_client_error(&self) -> bool {
        self.status_code()
            .map(|s| (400..500).contains(&s))
            .unwrap_or(false)
    }

    /// Check if this is a server error (5xx)
    pub fn is_server_error(&self) -> bool {
        self.status_code()
            .map(|s| (500..600).contains(&s))
            .unwrap_or(false)
    }

    /// Check if this is an authentication/authorization error (401 or 403)
    pub fn is_auth_error(&self) -> bool {
        self.status_code()
            .map(|s| s == 401 || s == 403)
            .unwrap_or(false)
    }

    /// Check if this is a timeout error
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout) || matches!(self, Self::Network(e) if e.is_timeout())
    }

    /// Check if this is a network connectivity error
    pub fn is_network(&self) -> bool {
        matches!(self, Self::Network(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_error_creation() {
        let err = HttpError::http(404, "Not found");
        assert_eq!(err.status_code(), Some(404));
        assert_eq!(err.to_string(), "HTTP 404: Not found");
    }

    #[test]
    fn test_from_status() {
        let err = HttpError::from_status(StatusCode::BAD_REQUEST, "Invalid input");
        assert_eq!(err.status_code(), Some(400));
        assert!(err.is_client_error());
    }

    #[test]
    fn test_is_client_error() {
        assert!(HttpError::http(400, "Bad Request").is_client_error());
        assert!(HttpError::http(404, "Not Found").is_client_error());
        assert!(HttpError::http(499, "Custom").is_client_error());
        assert!(!HttpError::http(500, "Server Error").is_client_error());
        assert!(!HttpError::http(200, "OK").is_client_error());
    }

    #[test]
    fn test_is_server_error() {
        assert!(HttpError::http(500, "Internal Server Error").is_server_error());
        assert!(HttpError::http(503, "Service Unavailable").is_server_error());
        assert!(!HttpError::http(404, "Not Found").is_server_error());
    }

    #[test]
    fn test_is_auth_error() {
        assert!(HttpError::http(401, "Unauthorized").is_auth_error());
        assert!(HttpError::http(403, "Forbidden").is_auth_error());
        assert!(!HttpError::http(400, "Bad Request").is_auth_error());
        assert!(!HttpError::http(404, "Not Found").is_auth_error());
    }

    #[test]
    fn test_timeout_error() {
        let err = HttpError::Timeout;
        assert!(err.is_timeout());
        assert_eq!(err.status_code(), None);
        assert_eq!(err.to_string(), "Request timeout");
    }

    #[test]
    fn test_parse_error() {
        let err = HttpError::ParseError("Invalid JSON".to_string());
        assert_eq!(err.status_code(), None);
        assert!(!err.is_client_error());
        assert!(!err.is_server_error());
        assert_eq!(err.to_string(), "Failed to parse response: Invalid JSON");
    }
}
