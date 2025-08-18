use super::{DeviceFlowSession, DeviceFlowStore, OAuthClient, OAuthError};
use base64::{engine::general_purpose, Engine as _};
use oauth2::Scope;
use std::time::SystemTime;
use tracing::{debug, error, info};

impl OAuthClient {
    pub async fn start_device_flow(
        &self,
        store: DeviceFlowStore,
    ) -> Result<DeviceFlowSession, OAuthError> {
        info!("Starting device flow with OIDC provider");
        debug!("OIDC Issuer URL: {}", self.oidc_issuer_url);

        // Request device and user codes from OIDC provider
        let details: oauth2::StandardDeviceAuthorizationResponse = self
            .client
            .exchange_device_code()
            .map_err(|e| {
                OAuthError::OAuth2(format!("Failed to create device auth request: {:?}", e))
            })?
            .add_scope(Scope::new("read_user".to_string()))
            .add_scope(Scope::new("read_api".to_string()))
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .request_async(oauth2::reqwest::async_http_client)
            .await
            .map_err(|e| {
                OAuthError::OAuth2(format!("Device authorization request failed: {:?}", e))
            })?;

        let expires_at = SystemTime::now() + details.expires_in();
        let device_code = details.device_code().secret().clone();
        let user_code = details.user_code().secret().clone();
        let verification_uri = details.verification_uri().to_string();

        let session = DeviceFlowSession {
            device_code: device_code.clone(),
            user_code: user_code.clone(),
            verification_uri: verification_uri.clone(),
            expires_at,
            oidc_access_token: None,
            completed: false,
            interval: details.interval().as_secs(),
        };

        // Store session
        {
            let mut sessions = store.lock().unwrap();
            sessions.insert(device_code.clone(), session.clone());
        }

        info!(
            "Device flow started - User code: {}, Verification URI: {}",
            user_code, verification_uri
        );

        Ok(session)
    }

    pub async fn poll_device_token(
        &self,
        device_code: &str,
        store: DeviceFlowStore,
    ) -> Result<String, OAuthError> {
        debug!("Polling for device token: {}", device_code);

        // Get session
        let session = {
            let sessions = store.lock().unwrap();
            sessions
                .get(device_code)
                .cloned()
                .ok_or(OAuthError::SessionNotFound)?
        };

        // Check if session is expired
        if SystemTime::now() > session.expires_at {
            error!("Device flow session expired");
            return Err(OAuthError::SessionExpired);
        }

        // Check if already completed
        if session.completed {
            if let Some(token) = session.oidc_access_token {
                debug!("Session already completed, returning cached token");
                return Ok(token);
            }
        }

        // Attempt to exchange the device code for an access token
        // This uses the stored device code to poll the OIDC provider
        match self.exchange_device_code_for_token(device_code).await {
            Ok(access_token) => {
                info!("Device flow completed successfully");

                // Update session
                {
                    let mut sessions = store.lock().unwrap();
                    if let Some(session) = sessions.get_mut(device_code) {
                        session.oidc_access_token = Some(access_token.clone());
                        session.completed = true;
                    }
                }

                Ok(access_token)
            }
            Err(e) => {
                debug!("Device flow polling result: {:?}", e);
                Err(e)
            }
        }
    }

    async fn exchange_device_code_for_token(
        &self,
        device_code: &str,
    ) -> Result<String, OAuthError> {
        debug!("Exchanging device code for token");

        // Create a device code token request to the OIDC provider
        let token_url = format!("{}/oauth/token", self.oidc_issuer_url);

        let params = [
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code),
        ];

        // Use the shared HTTP client - need to construct this manually since the shared client
        // doesn't support form data with basic auth directly
        let auth_header = format!(
            "Basic {}",
            general_purpose::STANDARD.encode(format!("{}:{}", self.client_id, self.client_secret))
        );

        let response = self
            .http_client
            .inner()
            .post(&token_url)
            .form(&params)
            .header("Authorization", auth_header)
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await?;

        debug!(
            "Token exchange response: status={}, body={}",
            status, response_text
        );

        if status.is_success() {
            // Parse the token response
            let token_response: serde_json::Value =
                serde_json::from_str(&response_text).map_err(OAuthError::Serde)?;

            if let Some(access_token) = token_response.get("access_token").and_then(|v| v.as_str())
            {
                Ok(access_token.to_string())
            } else {
                error!("No access_token in response: {}", response_text);
                Err(OAuthError::OAuth2(
                    "No access_token in response".to_string(),
                ))
            }
        } else if status == 400 {
            // Parse error response
            let error_response: Result<serde_json::Value, _> = serde_json::from_str(&response_text);
            if let Ok(error) = error_response {
                if let Some(error_code) = error.get("error").and_then(|v| v.as_str()) {
                    match error_code {
                        "authorization_pending" => {
                            debug!("Authorization still pending");
                            Err(OAuthError::AuthorizationPending)
                        }
                        "access_denied" => {
                            error!("Device flow access denied by user");
                            Err(OAuthError::AccessDenied)
                        }
                        "expired_token" => {
                            error!("Device code has expired");
                            Err(OAuthError::SessionExpired)
                        }
                        _ => {
                            error!("OAuth error: {}", error_code);
                            Err(OAuthError::OAuth2(format!("OAuth error: {}", error_code)))
                        }
                    }
                } else {
                    error!("Invalid error response format: {}", response_text);
                    Err(OAuthError::OAuth2(format!(
                        "Invalid error response: {}",
                        response_text
                    )))
                }
            } else {
                error!("Failed to parse error response: {}", response_text);
                Err(OAuthError::OAuth2(format!(
                    "Failed to parse error response: {}",
                    response_text
                )))
            }
        } else {
            error!(
                "Token exchange failed with status {}: {}",
                status, response_text
            );
            Err(OAuthError::OAuth2(format!(
                "Token exchange failed: HTTP {}",
                status
            )))
        }
    }

    pub async fn validate_oidc_token(&self, access_token: &str) -> Result<OidcUser, OAuthError> {
        debug!("Validating OIDC token");

        let user_url = format!("{}/oauth/userinfo", self.oidc_issuer_url);
        let response = self
            .http_client
            .inner()
            .get(&user_url)
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            error!("OIDC token validation failed: {}", response.status());
            return Err(OAuthError::Reqwest(
                response.error_for_status().unwrap_err(),
            ));
        }

        let user: OidcUser = response.json().await?;
        debug!(
            "OIDC user validated: {} <{}>",
            user.name.as_deref().unwrap_or("N/A"),
            user.email.as_deref().unwrap_or("N/A")
        );

        Ok(user)
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct OidcUser {
    #[serde(rename = "sub")]
    pub id: String, // OIDC subject is typically a string
    #[serde(rename = "preferred_username", default)]
    pub username: Option<String>, // Optional in OIDC
    #[serde(default)]
    pub name: Option<String>, // Optional in OIDC
    #[serde(default)]
    pub email: Option<String>, // Optional in OIDC
}
