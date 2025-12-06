use std::time::Duration;
use tokio::time::sleep;
use tracing::error;

use super::error::HttpError;

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: usize,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_delay_ms: 500,
            max_delay_ms: 8000, // 8 seconds
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RetryError {
    #[error("Exhausted all {attempts} retry attempts: {error}")]
    ExhaustedRetries { error: HttpError, attempts: usize },
    #[error("Non-retriable error: {0}")]
    NonRetriable(HttpError),
}

impl RetryError {
    /// Get the underlying HTTP error
    pub fn http_error(&self) -> &HttpError {
        match self {
            Self::ExhaustedRetries { error, .. } => error,
            Self::NonRetriable(error) => error,
        }
    }

    /// Get the HTTP status code if available
    pub fn status_code(&self) -> Option<u16> {
        self.http_error().status_code()
    }

    /// Check if this is an authentication/authorization error (401 or 403)
    pub fn is_auth_error(&self) -> bool {
        self.http_error().is_auth_error()
    }

    /// Check if this is a client error (4xx)
    pub fn is_client_error(&self) -> bool {
        self.http_error().is_client_error()
    }

    /// Check if this is a server error (5xx)
    pub fn is_server_error(&self) -> bool {
        self.http_error().is_server_error()
    }
}

/// Helper function to determine if an HTTP error is retriable
pub fn is_retriable_error(err: &HttpError) -> bool {
    match err {
        // Retry on server errors (5xx)
        HttpError::Http { status, .. } => (500..600).contains(status),
        // Retry on network/timeout errors
        HttpError::Network(e) => {
            e.is_timeout()
                || e.is_connect()
                || e.is_request()
                || e.status().is_some_and(|s| s.is_server_error())
        }
        HttpError::Timeout => true,
        // Don't retry parse errors (likely bad response format)
        HttpError::ParseError(_) => false,
    }
}

/// Helper function to execute a future with retry logic
pub async fn with_retry<F, Fut, T>(f: F, config: &RetryConfig) -> Result<T, RetryError>
where
    F: Fn() -> Fut + Clone,
    Fut: std::future::Future<Output = Result<T, HttpError>>,
{
    let mut retry_count = 0;
    let mut delay = config.initial_delay_ms;

    loop {
        match f().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                // Check if we've reached the max retries
                if retry_count >= config.max_retries - 1 {
                    return Err(RetryError::ExhaustedRetries {
                        error: err,
                        attempts: config.max_retries,
                    });
                }

                // Check if this error should be retried
                let should_retry = is_retriable_error(&err);

                if !should_retry {
                    return Err(RetryError::NonRetriable(err));
                }

                retry_count += 1;
                error!(
                    "API call failed (attempt {}/{}), retrying in {}ms: {}",
                    retry_count, config.max_retries, delay, err
                );

                // Sleep with exponential backoff
                sleep(Duration::from_millis(delay)).await;

                // Increase delay for next retry with exponential backoff (2x)
                // but cap it at MAX_RETRY_DELAY_MS
                delay = (delay * 2).min(config.max_delay_ms);
            }
        }
    }
}
