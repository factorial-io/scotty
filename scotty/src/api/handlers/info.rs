use axum::{debug_handler, extract::State, response::IntoResponse, Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::app_state::SharedAppState;
use scotty_core::settings::api_server::AuthMode;

#[derive(Serialize, ToSchema)]
pub struct OAuthConfig {
    pub enabled: bool,
    pub provider: String,
    pub redirect_url: String,
    pub oauth2_proxy_base_url: Option<String>,
    pub gitlab_url: Option<String>,
    pub client_id: Option<String>,
    pub device_flow_enabled: bool,
}

#[derive(Serialize, ToSchema)]
pub struct ServerInfo {
    pub domain: String,
    pub version: String,
    pub auth_mode: String,
    pub oauth_config: Option<OAuthConfig>,
}

#[utoipa::path(
    get,
    path = "/api/v1/info",
    responses(
        (status = 200, description = "Server information and configuration", body = ServerInfo)
    )
)]
#[debug_handler]
pub async fn info_handler(State(state): State<SharedAppState>) -> impl IntoResponse {
    let oauth_config = match state.settings.api.auth_mode {
        AuthMode::OAuth => Some(OAuthConfig {
            enabled: true,
            provider: "gitlab".to_string(),
            redirect_url: state.settings.api.oauth.redirect_url.clone(),
            // For native OAuth, use the server's own URL instead of oauth2-proxy URL
            oauth2_proxy_base_url: state
                .settings
                .api
                .oauth
                .oauth2_proxy_base_url
                .clone()
                .or_else(|| {
                    // Generate server URL from bind_address
                    let bind_addr = &state.settings.api.bind_address;
                    if bind_addr.starts_with("0.0.0.0:") {
                        // Replace 0.0.0.0 with localhost for client use
                        let port = bind_addr.split(':').nth(1).unwrap_or("21342");
                        Some(format!("http://localhost:{}", port))
                    } else if !bind_addr.starts_with("http") {
                        Some(format!("http://{}", bind_addr))
                    } else {
                        Some(bind_addr.clone())
                    }
                }),
            gitlab_url: state
                .settings
                .api
                .oauth
                .oidc_issuer_url
                .clone()
                .or_else(|| Some("https://gitlab.com".to_string())),
            client_id: state.settings.api.oauth.client_id.clone(),
            device_flow_enabled: state.settings.api.oauth.device_flow_enabled,
        }),
        _ => None,
    };

    let response = ServerInfo {
        domain: state.settings.apps.domain_suffix.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        auth_mode: match state.settings.api.auth_mode {
            AuthMode::Development => "dev".to_string(),
            AuthMode::OAuth => "oauth".to_string(),
            AuthMode::Bearer => "bearer".to_string(),
        },
        oauth_config,
    };

    Json(response)
}
