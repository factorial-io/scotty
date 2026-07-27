use super::rate_limiting::RateLimitingConfig;
use secrecy::SecretString;
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
    pub client_id: Option<String>,
    #[serde(skip_serializing)]
    pub client_secret: Option<SecretString>,
    #[serde(default = "default_device_flow_enabled")]
    pub device_flow_enabled: bool,
    /// DEPRECATED: use `api.base_url` instead.
    ///
    /// Frontend base URL for OAuth callback redirects. Kept only for backward
    /// compatibility: when `api.base_url` is unset, this value is used as a
    /// fallback and a deprecation warning is logged at startup.
    #[serde(default)]
    pub frontend_base_url: Option<String>,
}

impl Default for OAuthSettings {
    fn default() -> Self {
        Self {
            redirect_url: default_oauth_redirect_url(),
            oidc_issuer_url: None,
            client_id: None,
            client_secret: None,
            device_flow_enabled: default_device_flow_enabled(),
            frontend_base_url: None,
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
    #[serde(default)]
    pub rate_limiting: RateLimitingConfig,
    /// Public-facing base URL for Scotty (e.g., "https://scotty.example.com").
    /// Used by the landing page feature to redirect stopped-app requests
    /// back to Scotty's domain.
    #[serde(default)]
    pub base_url: Option<String>,
}

fn default_oauth_redirect_url() -> String {
    "/oauth2/start".to_string()
}

fn default_device_flow_enabled() -> bool {
    true
}

/// Default public base URL used when neither `api.base_url` nor the
/// deprecated `api.oauth.frontend_base_url` is configured.
pub const DEFAULT_BASE_URL: &str = "http://localhost:21342";

impl ApiServer {
    /// Resolve the public base URL of this Scotty installation.
    ///
    /// Every feature that needs Scotty's public URL (OAuth post-login
    /// redirects, the stopped-app landing page, own-domain detection) must go
    /// through this single resolution so they can never disagree on the
    /// origin. Resolution order: `api.base_url`, then the deprecated
    /// `api.oauth.frontend_base_url`, then [`DEFAULT_BASE_URL`]. Empty values
    /// count as unset; a trailing slash is stripped.
    pub fn public_base_url(&self) -> String {
        self.configured_base_url()
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
    }

    /// The explicitly configured public base URL, if any.
    pub fn configured_base_url(&self) -> Option<String> {
        [self.base_url.as_deref(), self.frontend_base_url()]
            .into_iter()
            .flatten()
            .map(str::trim)
            .find(|url| !url.is_empty())
            .map(|url| url.trim_end_matches('/').to_string())
    }

    /// Whether no public base URL is configured and the localhost default is
    /// in effect.
    pub fn is_using_default_base_url(&self) -> bool {
        self.configured_base_url().is_none()
    }

    fn frontend_base_url(&self) -> Option<&str> {
        self.oauth.frontend_base_url.as_deref()
    }

    /// Configuration problems around the public base URL, as human-readable
    /// warnings to be logged at startup.
    pub fn base_url_config_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        let frontend = self
            .frontend_base_url()
            .map(str::trim)
            .filter(|url| !url.is_empty());
        let base = self
            .base_url
            .as_deref()
            .map(str::trim)
            .filter(|url| !url.is_empty());

        if let Some(frontend) = frontend {
            match base {
                Some(base) if base.trim_end_matches('/') != frontend.trim_end_matches('/') => {
                    warnings.push(format!(
                        "api.oauth.frontend_base_url ('{}') differs from api.base_url ('{}'). \
                         api.base_url wins; remove the deprecated api.oauth.frontend_base_url \
                         setting to silence this warning.",
                        frontend, base
                    ));
                }
                _ => {
                    warnings.push(
                        "api.oauth.frontend_base_url is deprecated; set api.base_url instead."
                            .to_string(),
                    );
                }
            }
        }

        match self.configured_base_url() {
            None => {
                warnings.push(format!(
                    "api.base_url is not configured — falling back to the default '{}'. \
                     OAuth logins and the stopped-app landing page will not work correctly \
                     unless api.base_url is set to Scotty's public URL \
                     (e.g. 'https://scotty.example.com').",
                    DEFAULT_BASE_URL
                ));
            }
            Some(configured) => {
                let parses_with_host = url::Url::parse(&configured)
                    .map(|url| url.host_str().is_some())
                    .unwrap_or(false);
                if !parses_with_host {
                    warnings.push(format!(
                        "the configured public base URL ('{}') is not a valid absolute URL \
                         (missing 'https://' scheme?). OAuth redirects and the stopped-app \
                         landing page will not work until it is fixed.",
                        configured
                    ));
                }
            }
        }

        warnings
    }
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
            rate_limiting: RateLimitingConfig::default(),
            base_url: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn api_server(base_url: Option<&str>, frontend_base_url: Option<&str>) -> ApiServer {
        let mut api = ApiServer {
            base_url: base_url.map(str::to_string),
            ..Default::default()
        };
        api.oauth = OAuthSettings {
            frontend_base_url: frontend_base_url.map(str::to_string),
            ..Default::default()
        };
        api
    }

    #[test]
    fn test_public_base_url_prefers_base_url() {
        let api = api_server(
            Some("https://scotty.example.com"),
            Some("https://other.example.com"),
        );
        assert_eq!(api.public_base_url(), "https://scotty.example.com");
        assert!(!api.is_using_default_base_url());
    }

    #[test]
    fn test_public_base_url_falls_back_to_deprecated_frontend_base_url() {
        let api = api_server(None, Some("https://legacy.example.com"));
        assert_eq!(api.public_base_url(), "https://legacy.example.com");
        assert!(!api.is_using_default_base_url());
    }

    #[test]
    fn test_public_base_url_defaults_to_localhost() {
        let api = api_server(None, None);
        assert_eq!(api.public_base_url(), DEFAULT_BASE_URL);
        assert!(api.is_using_default_base_url());
    }

    #[test]
    fn test_public_base_url_treats_empty_as_unset_and_strips_trailing_slash() {
        let api = api_server(Some("  "), Some("https://legacy.example.com/"));
        assert_eq!(api.public_base_url(), "https://legacy.example.com");

        let api = api_server(Some("https://scotty.example.com/"), None);
        assert_eq!(api.public_base_url(), "https://scotty.example.com");
    }

    #[test]
    fn test_warnings_when_nothing_configured() {
        let warnings = api_server(None, None).base_url_config_warnings();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("api.base_url is not configured"));
    }

    #[test]
    fn test_no_warnings_when_base_url_configured() {
        let warnings =
            api_server(Some("https://scotty.example.com"), None).base_url_config_warnings();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_deprecation_warning_for_frontend_base_url() {
        let warnings =
            api_server(None, Some("https://scotty.example.com")).base_url_config_warnings();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("deprecated"));
    }

    #[test]
    fn test_conflict_warning_when_both_set_and_different() {
        let warnings = api_server(
            Some("https://scotty.example.com"),
            Some("http://scotty.example.com"),
        )
        .base_url_config_warnings();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("differs"));
        assert!(warnings[0].contains("api.base_url wins"));
    }

    #[test]
    fn test_warning_for_malformed_base_url() {
        // Missing scheme: parses as a relative/hostless URL and cannot be
        // used for redirects or own-domain detection.
        let warnings = api_server(Some("scotty.example.com"), None).base_url_config_warnings();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("not a valid absolute URL"));
    }

    #[test]
    fn test_deprecation_warning_when_both_set_and_equal() {
        let warnings = api_server(
            Some("https://scotty.example.com"),
            Some("https://scotty.example.com/"),
        )
        .base_url_config_warnings();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("deprecated"));
    }
}
