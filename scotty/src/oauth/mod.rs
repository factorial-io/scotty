pub mod cleanup;
pub mod client;
pub mod device_flow;
pub mod handlers;
pub mod metrics;
pub mod server;
pub mod store;

use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    DeviceAuthorizationUrl, EndpointMaybeSet, EndpointNotSet, EndpointSet, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenUrl,
};
use openidconnect::core::{
    CoreAuthenticationFlow, CoreClient, CoreProviderMetadata, CoreUserInfoClaims,
};
use openidconnect::{IssuerUrl, Nonce, OAuth2TokenResponse, TokenResponse as _};

/// Concrete `BasicClient` typestate for our configuration: auth URL, device
/// authorization URL and token URL are set; introspection and revocation are not.
type ConfiguredOAuthClient =
    BasicClient<EndpointSet, EndpointSet, EndpointNotSet, EndpointNotSet, EndpointSet>;
use scotty_core::utils::secret::MaskedSecret;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Clone)]
pub struct OAuthClient {
    pub client: ConfiguredOAuthClient,
    pub oidc_issuer_url: String,
    pub client_id: String,
    pub client_secret: SecretString,
    pub http_client: scotty_core::http::HttpClient,
    // Dedicated reqwest client from oauth2's bundled reqwest version, used for
    // oauth2 `request_async` calls. The workspace reqwest may differ in major
    // version from the one oauth2 implements `AsyncHttpClient` for, so we cannot
    // reuse `http_client`'s inner client here.
    oauth_http_client: oauth2::reqwest::Client,
}

// Custom Debug implementation to prevent leaking client_secret
impl std::fmt::Debug for OAuthClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthClient")
            .field("client", &self.client)
            .field("oidc_issuer_url", &self.oidc_issuer_url)
            .field("client_id", &self.client_id)
            .field("client_secret", &"[REDACTED]")
            .field("http_client", &self.http_client)
            .field("oauth_http_client", &self.oauth_http_client)
            .finish()
    }
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
    pub csrf_token: MaskedSecret,              // Protected CSRF token
    pub pkce_verifier: MaskedSecret,           // Protected PKCE verifier
    pub nonce: MaskedSecret,                   // OIDC nonce the ID token must carry
    pub redirect_url: String,                  // OAuth redirect URL for GitLab token exchange
    pub frontend_callback_url: Option<String>, // Frontend callback URL for final redirect
    pub expires_at: SystemTime,
}

// In-memory storage for web flow sessions
pub type WebFlowStore = Arc<Mutex<HashMap<String, WebFlowSession>>>;

// Temporary session for OAuth completion: the Scotty session token the
// frontend picks up once through `/oauth/exchange`.
#[derive(Debug, Clone)]
pub struct OAuthSession {
    pub token: TokenResponse,
    pub expires_at: SystemTime,
}

// In-memory storage for OAuth sessions (short-lived, for token exchange)
pub type OAuthSessionStore = Arc<Mutex<HashMap<String, OAuthSession>>>;

impl OAuthClient {
    pub fn new(
        client_id: String,
        client_secret: SecretString,
        oidc_issuer_url: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let auth_url = format!("{}/oauth/authorize", oidc_issuer_url);
        let token_url = format!("{}/oauth/token", oidc_issuer_url);
        let device_auth_url = format!("{}/oauth/authorize_device", oidc_issuer_url);

        let client = BasicClient::new(ClientId::new(client_id.clone()))
            .set_client_secret(ClientSecret::new(client_secret.expose_secret().to_string()))
            .set_auth_uri(AuthUrl::new(auth_url)?)
            .set_token_uri(TokenUrl::new(token_url)?)
            .set_device_authorization_url(DeviceAuthorizationUrl::new(device_auth_url)?);

        let http_client =
            scotty_core::http::HttpClient::with_timeout(std::time::Duration::from_secs(30))
                .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        // Disable redirects to prevent auth credentials from being forwarded on
        // redirect, matching the policy used by the shared HttpClient.
        let oauth_http_client = oauth2::reqwest::Client::builder()
            .redirect(oauth2::reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| format!("Failed to create OAuth HTTP client: {}", e))?;

        Ok(Self {
            client,
            oidc_issuer_url: oidc_issuer_url.clone(),
            client_id,
            client_secret,
            http_client,
            oauth_http_client,
        })
    }

    /// The provider's OIDC client, from discovery at `oidc_issuer_url`.
    /// ponytail: discovery (with JWKS) runs per login request, so key
    /// rotation needs no cache invalidation; cache it if login latency matters.
    async fn oidc_client(&self, redirect_url: String) -> Result<OidcClient, OAuthError> {
        let issuer = IssuerUrl::new(self.oidc_issuer_url.clone())
            .map_err(|e| OAuthError::UrlParse(e.to_string()))?;
        let metadata = CoreProviderMetadata::discover_async(issuer, &self.oauth_http_client)
            .await
            .map_err(|e| OAuthError::OAuth2(format!("OIDC discovery failed: {}", e)))?;
        let redirect_url =
            RedirectUrl::new(redirect_url).map_err(|e| OAuthError::UrlParse(e.to_string()))?;
        Ok(CoreClient::from_provider_metadata(
            metadata,
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(
                self.client_secret.expose_secret().to_string(),
            )),
        )
        .set_redirect_uri(redirect_url))
    }

    /// Generate the authorization URL for the web flow, with PKCE and a nonce.
    pub async fn get_authorization_url(
        &self,
        redirect_url: String,
        state: CsrfToken,
    ) -> Result<(url::Url, PkceCodeVerifier, Nonce), OAuthError> {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let client = self.oidc_client(redirect_url).await?;
        let (auth_url, _csrf_state, nonce) = client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                || state,
                Nonce::new_random,
            )
            .add_scope(Scope::new("profile".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();
        Ok((auth_url, pkce_verifier, nonce))
    }

    /// Exchange the authorization code and read the identity from the
    /// verified ID token (signature via JWKS, `iss`, `aud`, `nonce`, expiry).
    /// Userinfo is asked only when the ID token carries no email; without an
    /// email the login fails. The provider's tokens are dropped here.
    pub async fn login_identity(
        &self,
        code: AuthorizationCode,
        redirect_url: String,
        pkce_verifier: PkceCodeVerifier,
        nonce: Nonce,
    ) -> Result<LoginIdentity, OAuthError> {
        let client = self.oidc_client(redirect_url).await?;
        let token_response = client
            .exchange_code(code)
            .map_err(|e| OAuthError::OAuth2(e.to_string()))?
            .set_pkce_verifier(pkce_verifier)
            .request_async(&self.oauth_http_client)
            .await
            .map_err(|e| OAuthError::OAuth2(format!("Token exchange failed: {:?}", e)))?;
        let id_token = token_response
            .id_token()
            .ok_or_else(|| OAuthError::OAuth2("Provider returned no ID token".to_string()))?;
        let claims = id_token
            .claims(&client.id_token_verifier(), &nonce)
            .map_err(|e| OAuthError::InvalidRequest(format!("ID token rejected: {}", e)))?;

        let mut identity = LoginIdentity {
            user_id: claims
                .preferred_username()
                .map(|u| u.to_string())
                .unwrap_or_else(|| claims.subject().to_string()),
            name: claims
                .name()
                .and_then(|n| n.get(None))
                .map(|n| n.to_string()),
            picture: claims
                .picture()
                .and_then(|p| p.get(None))
                .map(|p| p.to_string()),
            email: verified_email(
                claims.email().map(|e| e.to_string()),
                claims.email_verified(),
            ),
        };

        if identity.email.is_none() {
            let info: CoreUserInfoClaims = client
                .user_info(
                    token_response.access_token().clone(),
                    Some(claims.subject().clone()),
                )
                .map_err(|e| OAuthError::OAuth2(e.to_string()))?
                .request_async(&self.oauth_http_client)
                .await
                .map_err(|e| OAuthError::OAuth2(format!("Userinfo request failed: {}", e)))?;
            identity.email =
                verified_email(info.email().map(|e| e.to_string()), info.email_verified());
            if identity.name.is_none() {
                identity.name = info.name().and_then(|n| n.get(None)).map(|n| n.to_string());
            }
        }

        if identity.email.is_none() {
            tracing::warn!("Login refused: the provider confirmed no email address");
            return Err(OAuthError::AccessDenied);
        }
        Ok(identity)
    }
}

/// The OIDC client after discovery: token and userinfo endpoints may be set.
type OidcClient = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

/// An email, unless the provider says it is unverified.
fn verified_email(email: Option<String>, verified: Option<bool>) -> Option<String> {
    email.filter(|_| verified != Some(false))
}

/// The identity a login confirmed.
#[derive(Debug, Clone)]
pub struct LoginIdentity {
    pub user_id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub picture: Option<String>,
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

// Use OAuthError from scotty-core
pub use scotty_core::auth::{OAuthError, TokenResponse};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_flow_session_debug_masks_secrets() {
        let session = WebFlowSession {
            csrf_token: MaskedSecret::new("csrf-secret-token-12345".to_string()),
            pkce_verifier: MaskedSecret::new("pkce-verifier-abcdefghijk".to_string()),
            nonce: MaskedSecret::new("nonce-value-xyz".to_string()),
            redirect_url: "https://example.com/oauth/callback".to_string(),
            frontend_callback_url: Some("https://app.example.com/callback".to_string()),
            expires_at: SystemTime::now() + std::time::Duration::from_secs(600),
        };

        let debug_output = format!("{:?}", session);

        // Verify full secrets are NOT in debug output
        assert!(
            !debug_output.contains("csrf-secret-token-12345"),
            "CSRF token should be masked in debug output"
        );
        assert!(
            !debug_output.contains("pkce-verifier-abcdefghijk"),
            "PKCE verifier should be masked in debug output"
        );

        // Verify output contains masked indicators (asterisks)
        assert!(
            debug_output.contains("***"),
            "Debug output should contain masking indicators"
        );

        // Verify last few characters are visible for debugging (MaskedSecret shows last 4 chars)
        assert!(
            debug_output.contains("2345") || debug_output.contains("345"),
            "Debug output should show last few characters for debugging"
        );
        assert!(
            debug_output.contains("hijk") || debug_output.contains("ijk"),
            "Debug output should show last few characters for debugging"
        );
    }
}
