use axum::{debug_handler, extract::State, response::IntoResponse, Json};

use crate::app_state::SharedAppState;
use scotty_core::api::{OAuthConfig, ServerInfo};
use scotty_core::settings::api_server::AuthMode;

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
            provider: "oidc".to_string(),
            redirect_url: state.settings.api.oauth.redirect_url.clone(),
            oidc_issuer_url: state.settings.api.oauth.oidc_issuer_url.clone(),
            client_id: state.settings.api.oauth.client_id.clone(),
            device_flow_enabled: state.settings.api.oauth.device_flow_enabled,
        }),
        _ => None,
    };

    let response = ServerInfo {
        domain: state.settings.apps.domain_suffix.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        auth_mode: state.settings.api.auth_mode.clone(),
        oauth_config,
        suisse_fonts_url: fonts_url(state.settings.api.suisse_fonts_url.as_deref()),
    };

    Json(response)
}

/// `api.suisse_fonts_url` without a trailing slash; empty counts as unset.
fn fonts_url(setting: Option<&str>) -> Option<String> {
    setting
        .map(|u| u.trim().trim_end_matches('/'))
        .filter(|u| !u.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::fonts_url;

    #[test]
    fn fonts_url_is_normalised_and_optional() {
        assert_eq!(fonts_url(None), None);
        assert_eq!(fonts_url(Some("")), None);
        assert_eq!(fonts_url(Some(" / ")), None);
        assert_eq!(
            fonts_url(Some("https://config.factorial.io/fonts/")),
            Some("https://config.factorial.io/fonts".to_string())
        );
    }
}
