use super::retry::{with_retry, RetryConfig, RetryError};
use anyhow::Context;
use futures_util::StreamExt;
use reqwest::{header::HeaderMap, Method, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

/// Maximum size of error response body to read (10KB)
/// This prevents potential DoS attacks via large error response bodies
const MAX_ERROR_BODY_SIZE: usize = 10 * 1024;

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
    ///
    /// Limits the body size to MAX_ERROR_BODY_SIZE to prevent DoS attacks
    async fn extract_error_message(response: Response) -> String {
        let status = response.status();

        // Read response body with size limit to prevent DoS attacks
        let mut body_bytes = Vec::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    // Check if adding this chunk would exceed the limit
                    if body_bytes.len() + chunk.len() > MAX_ERROR_BODY_SIZE {
                        // Body too large, stop reading and return status only
                        return format!("HTTP error: {}", status);
                    }
                    body_bytes.extend_from_slice(&chunk);
                }
                Err(_) => {
                    // If we can't read the body, just return the status
                    return format!("HTTP error: {}", status);
                }
            }
        }

        // Convert bytes to string
        match String::from_utf8(body_bytes) {
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
                // If body is not valid UTF-8, just return the status
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

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn test_extract_error_message_with_json() {
        // Test JSON response with message field
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/error"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_json(serde_json::json!({"message": "App name already exists"})),
            )
            .mount(&mock_server)
            .await;

        let client = HttpClient::new().unwrap();
        let response = client
            .get(&format!("{}/error", mock_server.uri()))
            .await
            .unwrap();

        let error_msg = HttpClient::extract_error_message(response).await;
        assert_eq!(error_msg, "400 Bad Request: App name already exists");
    }

    #[tokio::test]
    async fn test_extract_error_message_with_plain_text() {
        // Test plain text response (short body)
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/error"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Resource not found"))
            .mount(&mock_server)
            .await;

        let client = HttpClient::new().unwrap();
        let response = client
            .get(&format!("{}/error", mock_server.uri()))
            .await
            .unwrap();

        let error_msg = HttpClient::extract_error_message(response).await;
        assert_eq!(error_msg, "404 Not Found: Resource not found");
    }

    #[tokio::test]
    async fn test_extract_error_message_with_long_body() {
        // Test body longer than 500 characters but under MAX_ERROR_BODY_SIZE
        let mock_server = MockServer::start().await;
        let long_body = "x".repeat(600);

        Mock::given(method("GET"))
            .and(path("/error"))
            .respond_with(ResponseTemplate::new(500).set_body_string(long_body))
            .mount(&mock_server)
            .await;

        let client = HttpClient::new().unwrap();
        let response = client
            .get(&format!("{}/error", mock_server.uri()))
            .await
            .unwrap();

        let error_msg = HttpClient::extract_error_message(response).await;
        assert_eq!(error_msg, "HTTP error: 500 Internal Server Error");
    }

    #[tokio::test]
    async fn test_extract_error_message_with_excessive_body() {
        // Test body exceeding MAX_ERROR_BODY_SIZE (10KB)
        let mock_server = MockServer::start().await;
        let excessive_body = "x".repeat(15 * 1024); // 15KB

        Mock::given(method("GET"))
            .and(path("/error"))
            .respond_with(ResponseTemplate::new(500).set_body_string(excessive_body))
            .mount(&mock_server)
            .await;

        let client = HttpClient::new().unwrap();
        let response = client
            .get(&format!("{}/error", mock_server.uri()))
            .await
            .unwrap();

        let error_msg = HttpClient::extract_error_message(response).await;
        // Should stop reading at the limit and return only status
        assert_eq!(error_msg, "HTTP error: 500 Internal Server Error");
    }

    #[tokio::test]
    async fn test_extract_error_message_with_empty_body() {
        // Test empty body
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/error"))
            .respond_with(ResponseTemplate::new(403).set_body_string(""))
            .mount(&mock_server)
            .await;

        let client = HttpClient::new().unwrap();
        let response = client
            .get(&format!("{}/error", mock_server.uri()))
            .await
            .unwrap();

        let error_msg = HttpClient::extract_error_message(response).await;
        assert_eq!(error_msg, "HTTP error: 403 Forbidden");
    }

    #[tokio::test]
    async fn test_extract_error_message_with_json_no_message_field() {
        // Test JSON response without message field
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/error"))
            .respond_with(
                ResponseTemplate::new(422)
                    .set_body_json(serde_json::json!({"error": "Validation failed", "code": 1001})),
            )
            .mount(&mock_server)
            .await;

        let client = HttpClient::new().unwrap();
        let response = client
            .get(&format!("{}/error", mock_server.uri()))
            .await
            .unwrap();

        let error_msg = HttpClient::extract_error_message(response).await;
        // Should include the full JSON body
        assert!(error_msg.starts_with("422 Unprocessable Entity:"));
        assert!(error_msg.contains("Validation failed"));
    }

    #[tokio::test]
    async fn test_extract_error_message_with_malformed_json() {
        // Test malformed JSON (falls back to plain text)
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/error"))
            .respond_with(ResponseTemplate::new(400).set_body_string("{invalid json"))
            .mount(&mock_server)
            .await;

        let client = HttpClient::new().unwrap();
        let response = client
            .get(&format!("{}/error", mock_server.uri()))
            .await
            .unwrap();

        let error_msg = HttpClient::extract_error_message(response).await;
        assert_eq!(error_msg, "400 Bad Request: {invalid json");
    }
}
