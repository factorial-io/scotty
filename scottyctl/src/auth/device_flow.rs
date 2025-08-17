use super::{AuthError, OAuthConfig, StoredToken};
use serde::Deserialize;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;

#[derive(Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[allow(dead_code)]
    pub verification_uri_complete: Option<String>,
    #[allow(dead_code)]
    pub expires_in: u64,
    #[allow(dead_code)]
    pub interval: Option<u64>,
}

#[derive(Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[allow(dead_code)]
    pub token_type: String,
    #[allow(dead_code)]
    pub user_id: String,
    pub user_name: String,
    pub user_email: String,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
    error_description: Option<String>,
}

pub struct DeviceFlowClient {
    client: reqwest::Client,
    config: OAuthConfig,
    user_provided_server_url: String,
}

impl DeviceFlowClient {
    pub fn new(config: OAuthConfig, user_provided_server_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
            user_provided_server_url,
        }
    }

    pub async fn start_device_flow(&self) -> Result<DeviceCodeResponse, AuthError> {
        // Use Scotty's native device flow endpoint instead of calling OIDC provider directly
        let device_url = format!("{}/oauth/device", self.config.scotty_server_url);

        tracing::info!("Starting device flow with Scotty server");
        tracing::info!("Device URL: {}", device_url);

        let response = self
            .client
            .post(&device_url)
            .header("Accept", "application/json")
            .header("User-Agent", "scottyctl/1.0")
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!("Device flow request failed: {}", error_text);
            return Err(AuthError::ServerError);
        }

        let device_response: DeviceCodeResponse = response.json().await?;
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
                    // Continue polling
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

        let response = self
            .client
            .post(&token_url)
            .header("Accept", "application/json")
            .send()
            .await?;

        match response.status().as_u16() {
            200 => {
                let token: TokenResponse = response.json().await?;
                Ok(token)
            }
            400 => {
                // Check for "authorization_pending" error
                let error: ErrorResponse = response.json().await?;
                if error.error == "authorization_pending" {
                    Err(AuthError::AuthorizationPending)
                } else {
                    tracing::error!(
                        "Token request error: {} - {}",
                        error.error,
                        error.error_description.unwrap_or_default()
                    );
                    Err(AuthError::ServerError)
                }
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                tracing::error!(
                    "Token request failed with status {}: {}",
                    status,
                    error_text
                );
                Err(AuthError::ServerError)
            }
        }
    }
}
