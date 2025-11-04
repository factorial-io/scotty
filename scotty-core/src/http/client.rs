use super::retry::{with_retry, RetryConfig, RetryError};
use anyhow::Context;
use reqwest::{header::HeaderMap, Method, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

#[derive(Debug, Clone)]
pub struct HttpClient {
    client: reqwest::Client,
    default_timeout: Duration,
    retry_config: RetryConfig,
}

pub struct HttpClientBuilder {
    timeout: Option<Duration>,
    retry_config: Option<RetryConfig>,
    headers: Option<HeaderMap>,
}

impl Default for HttpClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClientBuilder {
    pub fn new() -> Self {
        Self {
            timeout: None,
            retry_config: None,
            headers: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = Some(retry_config);
        self
    }

    pub fn with_default_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }

    pub fn build(self) -> anyhow::Result<HttpClient> {
        let mut client_builder = reqwest::Client::builder();

        if let Some(timeout) = self.timeout {
            client_builder = client_builder.timeout(timeout);
        }

        if let Some(headers) = self.headers {
            client_builder = client_builder.default_headers(headers);
        }

        let client = client_builder.build()?;

        Ok(HttpClient {
            client,
            default_timeout: self.timeout.unwrap_or(Duration::from_secs(10)),
            retry_config: self.retry_config.unwrap_or_default(),
        })
    }
}

impl HttpClient {
    pub fn builder() -> HttpClientBuilder {
        HttpClientBuilder::new()
    }

    pub fn new() -> anyhow::Result<Self> {
        Self::builder().build()
    }

    pub fn with_timeout(timeout: Duration) -> anyhow::Result<Self> {
        Self::builder().with_timeout(timeout).build()
    }

    /// Helper function to extract error message from response body
    async fn extract_error_message(response: Response) -> String {
        let status = response.status();

        // Try to read the response body
        match response.text().await {
            Ok(body) => {
                // Try to parse as JSON and extract the "message" field
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(message) = json.get("message").and_then(|m| m.as_str()) {
                        return format!("{}: {}", status, message);
                    }
                }
                // If JSON parsing fails or no message field, return status + body
                if !body.is_empty() && body.len() < 500 {
                    return format!("{}: {}", status, body);
                }
                // Fallback to just status if body is too long or empty
                format!("HTTP error: {}", status)
            }
            Err(_) => {
                // If we can't read the body, just return the status
                format!("HTTP error: {}", status)
            }
        }
    }

    /// Make a GET request with retry logic
    pub async fn get(&self, url: &str) -> Result<Response, RetryError> {
        info!("GET request to {}", url);
        with_retry(
            || async {
                self.client
                    .get(url)
                    .timeout(self.default_timeout)
                    .send()
                    .await
                    .context("Failed to send GET request")
            },
            &self.retry_config,
        )
        .await
    }

    /// Make a GET request and deserialize JSON response with retry logic
    pub async fn get_json<T>(&self, url: &str) -> Result<T, RetryError>
    where
        T: for<'de> Deserialize<'de>,
    {
        with_retry(
            || async {
                let response = self
                    .client
                    .get(url)
                    .timeout(self.default_timeout)
                    .send()
                    .await
                    .context("Failed to send GET request")?;

                if !response.status().is_success() {
                    let error_msg = Self::extract_error_message(response).await;
                    return Err(anyhow::anyhow!("{}", error_msg));
                }

                let json = response
                    .json::<T>()
                    .await
                    .context("Failed to parse JSON response")?;

                Ok(json)
            },
            &self.retry_config,
        )
        .await
    }

    /// Make a POST request with retry logic
    pub async fn post<T>(&self, url: &str, body: &T) -> Result<Response, RetryError>
    where
        T: Serialize,
    {
        info!("POST request to {}", url);
        with_retry(
            || async {
                self.client
                    .post(url)
                    .timeout(self.default_timeout)
                    .json(body)
                    .send()
                    .await
                    .context("Failed to send POST request")
            },
            &self.retry_config,
        )
        .await
    }

    /// Make a POST request and deserialize JSON response with retry logic
    pub async fn post_json<T, R>(&self, url: &str, body: &T) -> Result<R, RetryError>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        with_retry(
            || async {
                let response = self
                    .client
                    .post(url)
                    .timeout(self.default_timeout)
                    .json(body)
                    .send()
                    .await
                    .context("Failed to send POST request")?;

                if !response.status().is_success() {
                    let error_msg = Self::extract_error_message(response).await;
                    return Err(anyhow::anyhow!("{}", error_msg));
                }

                let json = response
                    .json::<R>()
                    .await
                    .context("Failed to parse JSON response")?;

                Ok(json)
            },
            &self.retry_config,
        )
        .await
    }

    /// Make a request with custom method
    pub async fn request(&self, method: Method, url: &str) -> Result<Response, RetryError> {
        info!("{} request to {}", method, url);
        with_retry(
            || async {
                self.client
                    .request(method.clone(), url)
                    .timeout(self.default_timeout)
                    .send()
                    .await
                    .context("Failed to send request")
            },
            &self.retry_config,
        )
        .await
    }

    /// Make a request with custom method and body
    pub async fn request_with_body<T>(
        &self,
        method: Method,
        url: &str,
        body: &T,
    ) -> Result<Response, RetryError>
    where
        T: Serialize,
    {
        info!("{} request with body to {}", method, url);
        with_retry(
            || async {
                self.client
                    .request(method.clone(), url)
                    .timeout(self.default_timeout)
                    .json(body)
                    .send()
                    .await
                    .context("Failed to send request with body")
            },
            &self.retry_config,
        )
        .await
    }

    /// Get a reference to the underlying reqwest client for advanced usage
    pub fn inner(&self) -> &reqwest::Client {
        &self.client
    }

    /// Get the default timeout
    pub fn default_timeout(&self) -> Duration {
        self.default_timeout
    }

    /// Get the retry configuration
    pub fn retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }
}
