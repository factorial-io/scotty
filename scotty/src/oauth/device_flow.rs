use super::{DeviceFlowSession, DeviceFlowStore, OAuthClient, OAuthError};
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

        // For device flow polling, we need to store the device authorization response
        // For now, let's create a simple implementation that returns pending until completed
        // In a real implementation, you'd store the full device authorization response

        // Return authorization pending for now - this would be handled by the actual device flow
        Err(OAuthError::AuthorizationPending)

        // This code would be used when we have proper device auth response storage:
        /*
        match self
            .client
            .exchange_device_access_token(&device_auth_response)
            .request_async(oauth2::reqwest::async_http_client, tokio::time::sleep, None)
            .await
        {
            Ok(token) => {
                let access_token = token.access_token().secret().clone();
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
                let error_str = format!("{:?}", e);
                if error_str.contains("authorization_pending") {
                    debug!("Authorization pending, continue polling");
                    Err(OAuthError::AuthorizationPending)
                } else if error_str.contains("access_denied") {
                    error!("Device flow access denied by user");
                    Err(OAuthError::AccessDenied)
                } else {
                    error!("Device flow error: {:?}", e);
                    Err(OAuthError::OAuth2(error_str))
                }
            }
        }
        */
    }

    pub async fn validate_oidc_token(&self, access_token: &str) -> Result<OidcUser, OAuthError> {
        debug!("Validating OIDC token");

        let user_url = format!("{}/oauth/userinfo", self.oidc_issuer_url);
        let response = reqwest::Client::new()
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
