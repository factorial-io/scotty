use super::error::HttpError;
use super::retry::{with_retry, RetryConfig, RetryError};
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

    pub fn build(self) -> Result<HttpClient, HttpError> {
        let mut client_builder = reqwest::Client::builder()
            // Disable automatic redirects to prevent auth header stripping on http->https redirects
            // Instead, we detect redirects and return a helpful error message
            .redirect(reqwest::redirect::Policy::none());

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

    pub fn new() -> Result<Self, HttpError> {
        Self::builder().build()
    }

    pub fn with_timeout(timeout: Duration) -> Result<Self, HttpError> {
        Self::builder().with_timeout(timeout).build()
    }

    /// Check if a response is a redirect and return an appropriate error
    fn check_redirect(response: &Response) -> Option<HttpError> {
        let status = response.status();
        if status.is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|h| h.to_str().ok())
                .unwrap_or("unknown")
                .to_string();
            Some(HttpError::Redirect {
                status: status.as_u16(),
                location,
            })
        } else {
            None
        }
    }

    /// Read an error response body, bounded by MAX_ERROR_BODY_SIZE to prevent
    /// DoS via large bodies. `Err` carries the status-only fallback message.
    async fn read_error_body(response: Response) -> Result<String, String> {
        let status = response.status();
        let fallback = || format!("HTTP error: {}", status);
        let mut body_bytes = Vec::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|_| fallback())?;
            if body_bytes.len() + chunk.len() > MAX_ERROR_BODY_SIZE {
                return Err(fallback());
            }
            body_bytes.extend_from_slice(&chunk);
        }
        String::from_utf8(body_bytes).map_err(|_| fallback())
    }

    /// Turn a non-2xx response into an `HttpError`: `Busy` for a 409 whose body
    /// names the running operation, `Http` with the extracted message otherwise.
    pub async fn error_from_response(response: Response) -> HttpError {
        let status = response.status();
        let message = match Self::read_error_body(response).await {
            Ok(body) => {
                let json = serde_json::from_str::<serde_json::Value>(&body).ok();
                if status == reqwest::StatusCode::CONFLICT {
                    let operation = json
                        .as_ref()
                        .and_then(|j| j.get("operation"))
                        .and_then(|o| serde_json::from_value(o.clone()).ok());
                    if let Some(operation) = operation {
                        let message = json
                            .as_ref()
                            .and_then(|j| j.get("message")?.as_str())
                            .unwrap_or("app is busy")
                            .to_string();
                        return HttpError::Busy { message, operation };
                    }
                }
                match json.as_ref().and_then(|j| j.get("message")?.as_str()) {
                    Some(message) => format!("{}: {}", status, message),
                    None if !body.is_empty() && body.len() < 500 => {
                        format!("{}: {}", status, body)
                    }
                    None => format!("HTTP error: {}", status),
                }
            }
            Err(fallback) => fallback,
        };
        HttpError::http(status.as_u16(), message)
    }

    /// Helper function to extract error message from response body
    #[cfg(test)]
    async fn extract_error_message(response: Response) -> String {
        match Self::error_from_response(response).await {
            HttpError::Http { message, .. } => message,
            other => other.to_string(),
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
                    .map_err(HttpError::from)
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
                    .map_err(HttpError::from)?;

                // Check for redirects first - these should not be retried
                if let Some(redirect_err) = Self::check_redirect(&response) {
                    return Err(redirect_err);
                }

                if !response.status().is_success() {
                    return Err(Self::error_from_response(response).await);
                }

                let json = response
                    .json::<T>()
                    .await
                    .map_err(|e| HttpError::ParseError(e.to_string()))?;

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
                    .map_err(HttpError::from)
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
                    .map_err(HttpError::from)?;

                // Check for redirects first - these should not be retried
                if let Some(redirect_err) = Self::check_redirect(&response) {
                    return Err(redirect_err);
                }

                if !response.status().is_success() {
                    return Err(Self::error_from_response(response).await);
                }

                let json = response
                    .json::<R>()
                    .await
                    .map_err(|e| HttpError::ParseError(e.to_string()))?;

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
                    .map_err(HttpError::from)
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
                    .map_err(HttpError::from)
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
    async fn a_409_naming_an_operation_is_busy_and_not_retried() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/busy"))
            .respond_with(ResponseTemplate::new(409).set_body_json(serde_json::json!({
                "error": true,
                "message": "App shop is busy",
                "operation": {
                    "operation": "rebuild",
                    "task_id": null,
                    "started_at": "2026-11-12T10:03:11Z"
                }
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = HttpClient::new().unwrap();
        let err = client
            .get_json::<serde_json::Value>(&format!("{}/busy", mock_server.uri()))
            .await
            .unwrap_err();
        match err {
            RetryError::NonRetriable(HttpError::Busy { message, operation }) => {
                assert_eq!(message, "App shop is busy");
                assert_eq!(operation.operation, "rebuild");
            }
            other => panic!("expected Busy, got {other:?}"),
        }
    }

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

    #[tokio::test]
    async fn test_redirect_returns_error() {
        // Test that redirects are not followed and return a helpful error
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api"))
            .respond_with(
                ResponseTemplate::new(301)
                    .insert_header("Location", "https://secure.example.com/api"),
            )
            .mount(&mock_server)
            .await;

        let client = HttpClient::new().unwrap();
        let result: Result<serde_json::Value, _> =
            client.get_json(&format!("{}/api", mock_server.uri())).await;

        assert!(result.is_err());
        let err = result.unwrap_err();

        // Should be a non-retriable redirect error
        match err {
            crate::http::RetryError::NonRetriable(http_err) => {
                assert!(http_err.is_redirect());
                assert_eq!(
                    http_err.redirect_location(),
                    Some("https://secure.example.com/api")
                );
                assert!(http_err
                    .to_string()
                    .contains("Please update your server URL"));
            }
            _ => panic!("Expected NonRetriable redirect error, got: {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_redirect_not_retried() {
        // Ensure redirects are not retried
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let mock_server = MockServer::start().await;
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        Mock::given(method("GET"))
            .and(path("/api"))
            .respond_with(move |_: &wiremock::Request| {
                call_count_clone.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(302)
                    .insert_header("Location", "https://other.example.com/api")
            })
            .mount(&mock_server)
            .await;

        let client = HttpClient::new().unwrap();
        let result: Result<serde_json::Value, _> =
            client.get_json(&format!("{}/api", mock_server.uri())).await;

        assert!(result.is_err());
        // Should only be called once - no retries for redirects
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }
}
