use super::{AuthError, OAuthConfig, StoredToken};
use scotty_core::auth::{DeviceFlowResponse, ErrorResponse, TokenResponse};
use scotty_core::http::HttpClient;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;

pub struct DeviceFlowClient {
    client: HttpClient,
    config: OAuthConfig,
    user_provided_server_url: String,
}

impl DeviceFlowClient {
    pub fn new(config: OAuthConfig, user_provided_server_url: String) -> Result<Self, AuthError> {
        let client = HttpClient::with_timeout(Duration::from_secs(30))
            .map_err(|_| AuthError::ServerError)?;

        Ok(Self {
            client,
            config,
            user_provided_server_url,
        })
    }

    pub async fn start_device_flow(&self) -> Result<DeviceFlowResponse, AuthError> {
        // Use Scotty's native device flow endpoint instead of calling OIDC provider directly
        let device_url = format!("{}/oauth/device", self.config.scotty_server_url);

        tracing::info!("Starting device flow with Scotty server");
        tracing::info!("Device URL: {}", device_url);

        let device_response = self
            .client
            .post_json::<serde_json::Value, DeviceFlowResponse>(&device_url, &serde_json::json!({}))
            .await
            .map_err(|_| AuthError::ServerError)?;

        Ok(device_response)
    }

    pub async fn poll_for_token(
        &self,
        device_code: &str,
        timeout_seconds: u64,
    ) -> Result<StoredToken, AuthError> {
        let start_time = SystemTime::now();
        let timeout_duration = Duration::from_secs(timeout_seconds);
        let poll_interval = Duration::from_secs(5); // Default polling interval

        loop {
            if start_time.elapsed().unwrap_or_default() > timeout_duration {
                return Err(AuthError::Timeout);
            }

            match self.try_get_token(device_code).await {
                Ok(token_response) => {
                    return Ok(StoredToken {
                        access_token: token_response.access_token,
                        refresh_token: None, // Device flow response doesn't include refresh token
                        expires_at: None,    // Device flow response doesn't include expiration
                        user_email: token_response.user_email,
                        user_name: token_response.user_name,
                        server_url: self.user_provided_server_url.clone(),
                    });
                }
                Err(AuthError::AuthorizationPending) => {
                    // Continue polling - check if this was a slow_down by looking at the detailed error
                    // If the server returned slow_down, the interval should be increased
                    sleep(poll_interval).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn try_get_token(&self, device_code: &str) -> Result<TokenResponse, AuthError> {
        let token_url = format!(
            "{}/oauth/device/token?device_code={}",
            self.config.scotty_server_url, device_code
        );

        // Try to get the token, the shared HTTP client will handle errors
        match self
            .client
            .post_json::<serde_json::Value, TokenResponse>(&token_url, &serde_json::json!({}))
            .await
        {
            Ok(token) => Ok(token),
            Err(_) => {
                // If it fails, try to get detailed error information
                match self.client.post(&token_url, &serde_json::json!({})).await {
                    Ok(response) => {
                        let status = response.status().as_u16();
                        match status {
                            400 | 401 | 403 | 404 | 429 => {
                                // Parse the error response for OAuth errors
                                match response.json::<ErrorResponse>().await {
                                    Ok(error) => {
                                        tracing::debug!(
                                            "OAuth error response: {} - {}",
                                            error.error,
                                            error
                                                .error_description
                                                .as_deref()
                                                .unwrap_or("No description")
                                        );

                                        // Handle specific OAuth errors
                                        match error.error.as_str() {
                                            "authorization_pending" => {
                                                Err(AuthError::AuthorizationPending)
                                            }
                                            "slow_down" => {
                                                tracing::debug!("Server requested slow down");
                                                Err(AuthError::AuthorizationPending)
                                                // Treat as continue polling
                                            }
                                            "access_denied" => {
                                                tracing::error!("User denied authorization");
                                                Err(AuthError::ServerError)
                                            }
                                            "session_not_found" => {
                                                tracing::error!("Device flow session not found");
                                                Err(AuthError::ServerError)
                                            }
                                            "expired_token" | "expired_session" => {
                                                tracing::error!("Device flow session expired");
                                                Err(AuthError::Timeout)
                                            }
                                            _ => {
                                                tracing::error!(
                                                    "Token request error: {} - {}",
                                                    error.error,
                                                    error.error_description.unwrap_or_default()
                                                );
                                                Err(AuthError::ServerError)
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "Failed to parse error response (status {}): {}",
                                            status,
                                            e
                                        );
                                        Err(AuthError::ServerError)
                                    }
                                }
                            }
                            _ => {
                                tracing::error!("Unexpected HTTP status: {}", status);
                                Err(AuthError::ServerError)
                            }
                        }
                    }
                    Err(_) => Err(AuthError::ServerError),
                }
            }
        }
    }
}
