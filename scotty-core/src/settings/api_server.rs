use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Default development user identifier
/// This uses a URI-like format that cannot exist in real OAuth/OIDC systems,
/// preventing accidental privilege escalation in production environments.
pub const DEFAULT_DEV_USER_EMAIL: &str = "dev:system:internal";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default, ToSchema)]
pub enum AuthMode {
    #[serde(rename = "dev")]
    Development,
    #[serde(rename = "oauth")]
    OAuth,
    #[serde(rename = "bearer")]
    #[default]
    Bearer,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
#[readonly::make]
pub struct OAuthSettings {
    #[serde(default = "default_oauth_redirect_url")]
    pub redirect_url: String,
    pub oidc_issuer_url: Option<String>,
    pub oauth2_proxy_base_url: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    #[serde(default = "default_device_flow_enabled")]
    pub device_flow_enabled: bool,
    /// Frontend base URL for OAuth callback redirects
    /// Example: "http://localhost:21342" or "https://scotty.example.com"
    #[serde(default = "default_frontend_base_url")]
    pub frontend_base_url: String,
}

impl Default for OAuthSettings {
    fn default() -> Self {
        Self {
            redirect_url: default_oauth_redirect_url(),
            oidc_issuer_url: None,
            oauth2_proxy_base_url: None,
            client_id: None,
            client_secret: None,
            device_flow_enabled: default_device_flow_enabled(),
            frontend_base_url: default_frontend_base_url(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
#[readonly::make]
pub struct ApiServer {
    pub bind_address: String,
    pub access_token: Option<String>,
    #[serde(deserialize_with = "deserialize_bytes")]
    pub create_app_max_size: usize,
    #[serde(default)]
    pub auth_mode: AuthMode,
    pub dev_user_email: Option<String>,
    pub dev_user_name: Option<String>,
    #[serde(default)]
    pub oauth: OAuthSettings,
    #[serde(default)]
    pub bearer_tokens: HashMap<String, String>,
}

fn default_oauth_redirect_url() -> String {
    "/oauth2/start".to_string()
}

fn default_device_flow_enabled() -> bool {
    true
}

fn default_frontend_base_url() -> String {
    "http://localhost:21342".to_string()
}

impl Default for ApiServer {
    fn default() -> Self {
        ApiServer {
            bind_address: "0.0.0.0:21342".to_string(),
            access_token: None,
            create_app_max_size: 1024 * 1024 * 10,
            auth_mode: AuthMode::default(),
            dev_user_email: Some(DEFAULT_DEV_USER_EMAIL.to_string()),
            dev_user_name: Some("Dev User".to_string()),
            oauth: OAuthSettings::default(),
            bearer_tokens: HashMap::new(),
        }
    }
}

fn deserialize_bytes<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let s = s.trim().to_uppercase();

    let (num_part, suffix) = s.split_at(s.len().saturating_sub(1));
    let multiplier = match suffix {
        "G" => 1_024 * 1_024 * 1_024,
        "M" => 1_024 * 1_024,
        "K" => 1_024,
        _ => 1,
    };

    let num: usize = num_part.parse().map_err(serde::de::Error::custom)?;
    Ok(num * multiplier)
}
