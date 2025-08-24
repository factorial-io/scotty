use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::settings::api_server::AuthMode;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OAuthConfig {
    pub enabled: bool,
    pub provider: String,
    pub redirect_url: String,
    pub oauth2_proxy_base_url: Option<String>,
    pub oidc_issuer_url: Option<String>,
    pub client_id: Option<String>,
    pub device_flow_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ServerInfo {
    pub domain: String,
    pub version: String,
    #[serde(default)]
    pub auth_mode: AuthMode,
    pub oauth_config: Option<OAuthConfig>,
}
