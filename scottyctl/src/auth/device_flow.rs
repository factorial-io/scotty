use super::{AuthError, OAuthConfig, StoredToken};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use tokio::time::sleep;

#[derive(Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: Option<u64>,
}

#[derive(Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub token_type: String,
}

#[derive(Deserialize)]
struct GitLabUser {
    email: String,
    name: String,
    username: String,
}

#[derive(Serialize, Debug)]
struct DeviceCodeRequest {
    client_id: String,
    scope: String,
}

#[derive(Serialize, Debug)]
struct TokenRequest {
    grant_type: String,
    device_code: String,
    client_id: String,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
    error_description: Option<String>,
}

pub struct DeviceFlowClient {
    client: reqwest::Client,
    config: OAuthConfig,
}

impl DeviceFlowClient {
    pub fn new(config: OAuthConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    pub async fn start_device_flow(&self) -> Result<DeviceCodeResponse, AuthError> {
        // Use Scotty's native device flow endpoint instead of GitLab directly
        let device_url = format!("{}/oauth/device", self.config.oauth2_proxy_base_url);

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
                    // Get user info from the obtained token
                    let user_info = self.get_user_info(&token_response.access_token).await?;

                    return Ok(StoredToken {
                        access_token: token_response.access_token,
                        refresh_token: token_response.refresh_token,
                        expires_at: token_response
                            .expires_in
                            .map(|secs| SystemTime::now() + Duration::from_secs(secs)),
                        user_email: user_info.email,
                        user_name: user_info.name,
                        server_url: self.config.oauth2_proxy_base_url.clone(),
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
        let token_url = format!("{}/oauth/device/token", self.config.oauth2_proxy_base_url);

        let request = TokenRequest {
            grant_type: "urn:ietf:params:oauth:grant-type:device_code".to_string(),
            device_code: device_code.to_string(),
            client_id: self.config.client_id.clone(),
        };

        let response = self
            .client
            .post(&token_url)
            .json(&request)
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

    async fn get_user_info(&self, access_token: &str) -> Result<GitLabUser, AuthError> {
        // Use Scotty's validate-token endpoint to get user info
        let user_url = format!(
            "{}/api/v1/authenticated/validate-token",
            self.config.oauth2_proxy_base_url
        );

        let response = self
            .client
            .post(&user_url)
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AuthError::TokenValidationFailed);
        }

        // Scotty's validate-token should return user info in the response
        // For now, we'll create a placeholder user since the actual response format might be different
        // TODO: Update this once we know the exact format of Scotty's validate-token response
        let user = GitLabUser {
            email: "oauth-user@example.com".to_string(),
            name: "OAuth User".to_string(),
            username: "oauth-user".to_string(),
        };
        Ok(user)
    }
}
