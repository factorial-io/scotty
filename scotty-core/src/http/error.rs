//! HTTP error types that preserve status code information throughout the error chain.
//!
//! This module provides custom error types for the HTTP client that maintain HTTP
//! status codes, enabling type-safe error handling without string parsing.
//!
//! # Migration from anyhow::Error
//!
//! Before:
//! ```ignore
//! let result: Result<T, anyhow::Error> = client.get_json(url).await;
//! // Had to parse error strings to get status codes
//! if err.to_string().contains("401") { ... }
//! ```
//!
//! After:
//! ```ignore
//! let result: Result<T, RetryError> = client.get_json(url).await;
//! if let Err(RetryError::NonRetriable(http_err)) = result {
//!     if http_err.is_auth_error() { ... }
//! }
//! ```
//!
//! # Examples
//!
//! ```
//! use scotty_core::http::HttpError;
//!
//! // Create an HTTP error
//! let err = HttpError::http(404, "Not found");
//! assert_eq!(err.status_code(), Some(404));
//! assert!(err.is_client_error());
//!
//! // Check for specific error types
//! if err.is_auth_error() {
//!     // Handle 401/403
//! }
//! ```

use reqwest::StatusCode;

/// HTTP client error types that preserve status code information
#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    /// HTTP error response with status code and message
    #[error("HTTP {status}: {message}")]
    Http { status: u16, message: String },

    /// Network-level error (connection, DNS, etc.)
    #[error("Network error: {0}")]
    Network(reqwest::Error),

    /// Request timeout
    #[error("Request timeout")]
    Timeout,

    /// Failed to parse response body
    #[error("Failed to parse response: {0}")]
    ParseError(String),

    /// Server returned a redirect response
    #[error("Server returned a {status} redirect to {location}. Please update your server URL to use the correct address.")]
    Redirect { status: u16, location: String },
}

impl From<reqwest::Error> for HttpError {
    fn from(err: reqwest::Error) -> Self {
        // If the reqwest error has a status code, preserve it as Http variant
        if let Some(status) = err.status() {
            Self::Http {
                status: status.as_u16(),
                message: err.to_string(),
            }
        } else if err.is_timeout() {
            Self::Timeout
        } else {
            Self::Network(err)
        }
    }
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

    /// Check if this is a redirect error
    pub fn is_redirect(&self) -> bool {
        matches!(self, Self::Redirect { .. })
    }

    /// Get the redirect location if this is a redirect error
    pub fn redirect_location(&self) -> Option<&str> {
        match self {
            Self::Redirect { location, .. } => Some(location),
            _ => None,
        }
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

    #[test]
    fn test_redirect_error() {
        let err = HttpError::Redirect {
            status: 301,
            location: "https://example.com/api".to_string(),
        };
        assert!(err.is_redirect());
        assert_eq!(err.redirect_location(), Some("https://example.com/api"));
        assert!(!err.is_client_error());
        assert!(!err.is_server_error());
        assert!(err.to_string().contains("301 redirect"));
        assert!(err.to_string().contains("https://example.com/api"));
        assert!(err.to_string().contains("Please update your server URL"));
    }

    #[test]
    fn test_from_reqwest_error_with_status() {
        // Create a mock server to generate a reqwest error with status
        use wiremock::{
            matchers::{method, path},
            Mock, MockServer, ResponseTemplate,
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mock_server = MockServer::start().await;

            Mock::given(method("GET"))
                .and(path("/test"))
                .respond_with(ResponseTemplate::new(404).set_body_string("Not found"))
                .mount(&mock_server)
                .await;

            let client = reqwest::Client::new();
            let url = format!("{}/test", mock_server.uri());
            let response = client.get(url).send().await.unwrap();

            // Get the status before converting to error
            assert_eq!(response.status(), 404);

            // Create error from response (status already consumed, so this creates a different error)
            // Instead, test with error_for_status which preserves the status
            let err = response.error_for_status().unwrap_err();
            let http_err: HttpError = err.into();

            // Should be Http variant with status code preserved
            assert_eq!(http_err.status_code(), Some(404));
            assert!(http_err.is_client_error());
            assert!(matches!(http_err, HttpError::Http { .. }));
        });
    }
}
