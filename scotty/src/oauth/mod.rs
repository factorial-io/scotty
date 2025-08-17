pub mod client;
pub mod device_flow;
pub mod handlers;

use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    DeviceAuthorizationUrl, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
    TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct OAuthClient {
    pub client: BasicClient,
    pub oidc_issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceFlowSession {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_at: SystemTime,
    pub oidc_access_token: Option<String>,
    pub completed: bool,
    // Store the interval from device auth response for proper polling
    pub interval: u64,
}

// In-memory storage for device flow sessions
// In production, this should use Redis or database
pub type DeviceFlowStore = Arc<Mutex<HashMap<String, DeviceFlowSession>>>;

// Web flow session for PKCE storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFlowSession {
    pub csrf_token: String,
    pub pkce_verifier: String,                 // Base64 encoded for storage
    pub redirect_url: String,                  // OAuth redirect URL for GitLab token exchange
    pub frontend_callback_url: Option<String>, // Frontend callback URL for final redirect
    pub expires_at: SystemTime,
}

// In-memory storage for web flow sessions
pub type WebFlowStore = Arc<Mutex<HashMap<String, WebFlowSession>>>;

// Temporary session for OAuth completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthSession {
    pub oidc_token: String,
    pub user: crate::oauth::device_flow::OidcUser,
    pub expires_at: SystemTime,
}

// In-memory storage for OAuth sessions (short-lived, for token exchange)
pub type OAuthSessionStore = Arc<Mutex<HashMap<String, OAuthSession>>>;

impl OAuthClient {
    pub fn new(
        client_id: String,
        client_secret: String,
        oidc_issuer_url: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let auth_url = format!("{}/oauth/authorize", oidc_issuer_url);
        let token_url = format!("{}/oauth/token", oidc_issuer_url);
        let device_auth_url = format!("{}/oauth/authorize_device", oidc_issuer_url);

        let client = BasicClient::new(
            ClientId::new(client_id.clone()),
            Some(ClientSecret::new(client_secret.clone())),
            AuthUrl::new(auth_url)?,
            Some(TokenUrl::new(token_url)?),
        )
        .set_device_authorization_url(DeviceAuthorizationUrl::new(device_auth_url)?);

        Ok(Self {
            client,
            oidc_issuer_url: oidc_issuer_url.clone(),
            client_id,
            client_secret,
        })
    }

    /// Generate authorization URL for web flow
    pub fn get_authorization_url(
        &self,
        redirect_url: String,
        state: CsrfToken,
    ) -> Result<(url::Url, PkceCodeVerifier), OAuthError> {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // Store PKCE verifier for later use - in a real implementation you'd store this securely
        // For now, we'll include it in the state parameter (not recommended for production)

        let redirect_url = RedirectUrl::new(redirect_url).map_err(OAuthError::Url)?;

        let client = self.client.clone().set_redirect_uri(redirect_url);

        let (auth_url, _csrf_state) = client
            .authorize_url(|| state)
            .add_scope(Scope::new("read_user".to_string()))
            .add_scope(Scope::new("read_api".to_string()))
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .set_pkce_challenge(pkce_challenge.clone())
            .url();

        Ok((auth_url, pkce_verifier))
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code_for_token(
        &self,
        code: AuthorizationCode,
        redirect_url: String,
        pkce_verifier: PkceCodeVerifier,
    ) -> Result<String, OAuthError> {
        let redirect_url = RedirectUrl::new(redirect_url).map_err(OAuthError::Url)?;

        let client = self.client.clone().set_redirect_uri(redirect_url);

        let token_result = client
            .exchange_code(code)
            .set_pkce_verifier(pkce_verifier)
            .request_async(oauth2::reqwest::async_http_client)
            .await
            .map_err(|e| OAuthError::OAuth2(format!("Token exchange failed: {:?}", e)))?;

        // Extract access token
        let access_token = token_result.access_token().secret().clone();

        Ok(access_token)
    }
}

pub fn create_device_flow_store() -> DeviceFlowStore {
    Arc::new(Mutex::new(HashMap::new()))
}

pub fn create_web_flow_store() -> WebFlowStore {
    Arc::new(Mutex::new(HashMap::new()))
}

pub fn create_oauth_session_store() -> OAuthSessionStore {
    Arc::new(Mutex::new(HashMap::new()))
}

#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("OAuth2 error: {0}")]
    OAuth2(String),
    #[error("HTTP error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),
    #[error("Device flow session not found")]
    SessionNotFound,
    #[error("Device flow session expired")]
    SessionExpired,
    #[error("Authorization pending")]
    AuthorizationPending,
    #[error("Device flow denied")]
    AccessDenied,
}
