use super::{AuthError, OAuthConfig};
use crate::context::ServerSettings;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ServerInfo {
    pub domain: String,
    pub version: String,
    pub auth_mode: String,
    pub oauth_config: Option<OAuthConfigResponse>,
}

#[derive(Deserialize)]
pub struct OAuthConfigResponse {
    pub enabled: bool,
    pub provider: String,
    pub redirect_url: String,
    pub oauth2_proxy_base_url: Option<String>,
    pub oidc_issuer_url: Option<String>,
    pub client_id: Option<String>,
    pub device_flow_enabled: bool,
}

pub async fn get_server_info(server: &ServerSettings) -> Result<ServerInfo, AuthError> {
    let url = format!("{}/api/v1/info", server.server);

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(AuthError::ServerError);
    }

    let server_info: ServerInfo = response.json().await?;
    Ok(server_info)
}

pub fn server_info_to_oauth_config(server_info: ServerInfo) -> Result<OAuthConfig, AuthError> {
    match server_info.oauth_config {
        Some(oauth_config) if oauth_config.enabled => {
            if !oauth_config.device_flow_enabled {
                return Err(AuthError::DeviceFlowNotEnabled);
            }

            let oauth2_proxy_base_url = oauth_config
                .oauth2_proxy_base_url
                .ok_or(AuthError::InvalidResponse)?;
            let oidc_issuer_url = oauth_config
                .oidc_issuer_url
                .unwrap_or_else(|| "https://gitlab.com".to_string());
            let client_id = oauth_config.client_id.ok_or(AuthError::InvalidResponse)?;

            Ok(OAuthConfig {
                enabled: true,
                provider: oauth_config.provider,
                oauth2_proxy_base_url,
                oidc_issuer_url,
                client_id,
                device_flow_enabled: oauth_config.device_flow_enabled,
            })
        }
        Some(_) => Err(AuthError::DeviceFlowNotEnabled),
        None => Err(AuthError::OAuthNotConfigured),
    }
}
