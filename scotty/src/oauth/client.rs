use super::{OAuthClient, OAuthError};
use scotty_core::settings::api_server::OAuthSettings;

pub fn create_oauth_client(
    oauth_config: &OAuthSettings,
) -> Result<Option<OAuthClient>, OAuthError> {
    // Check if we have the required OAuth configuration
    let client_id = match &oauth_config.client_id {
        Some(id) => id.clone(),
        None => return Ok(None), // OAuth not configured
    };

    let client_secret = match &oauth_config.client_secret {
        Some(secret) => secret.clone(),
        None => return Ok(None), // OAuth not configured
    };

    let oidc_issuer_url = oauth_config
        .oidc_issuer_url
        .clone()
        .unwrap_or_else(|| "https://gitlab.com".to_string());

    match OAuthClient::new(client_id, client_secret, oidc_issuer_url) {
        Ok(client) => {
            tracing::info!("OAuth client initialized successfully");
            Ok(Some(client))
        }
        Err(e) => {
            tracing::error!("Failed to create OAuth client: {}", e);
            Err(OAuthError::UrlParse("Empty host".to_string())) // Convert to our error type
        }
    }
}
