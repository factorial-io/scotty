use std::time::Duration;
use tokio::time::sleep;
use tracing::error;

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
    #[error("Exhausted all retry attempts: {0}")]
    ExhaustedRetries(anyhow::Error),
    #[error("Non-retriable error: {0}")]
    NonRetriable(anyhow::Error),
}

/// Helper function to determine if an error is retriable
pub fn is_retriable_error(err: &reqwest::Error) -> bool {
    err.is_timeout()
        || err.is_connect()
        || err.is_request()
        || err.status().is_some_and(|s| s.is_server_error())
}

/// Helper function to execute a future with retry logic
pub async fn with_retry<F, Fut, T>(f: F, config: &RetryConfig) -> Result<T, RetryError>
where
    F: Fn() -> Fut + Clone,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let mut retry_count = 0;
    let mut delay = config.initial_delay_ms;

    loop {
        match f().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                // Check if we've reached the max retries
                if retry_count >= config.max_retries - 1 {
                    return Err(RetryError::ExhaustedRetries(err));
                }

                // Check if it's a reqwest error that we should retry
                let should_retry = if let Some(reqwest_err) = err.downcast_ref::<reqwest::Error>() {
                    is_retriable_error(reqwest_err)
                } else {
                    // Also retry on JSON parsing errors which might be due to partial responses
                    err.to_string().contains("Failed to parse")
                };

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
