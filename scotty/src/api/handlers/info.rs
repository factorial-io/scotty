use axum::{debug_handler, extract::State, response::IntoResponse, Json};

use crate::app_state::SharedAppState;

#[utoipa::path(
    get,
    path = "/api/v1/info",
    responses(
    (status = 200, description = "Some global info of the running server.")
    )
)]
#[debug_handler]
pub async fn info_handler(State(state): State<SharedAppState>) -> impl IntoResponse {
    let json_response = serde_json::json!({
        "domain": state.settings.apps.domain_suffix.clone(),
        "version": env!("CARGO_PKG_VERSION"),
        "auth_mode": match state.settings.api.auth_mode {
            scotty_core::settings::api_server::AuthMode::Development => "dev",
            scotty_core::settings::api_server::AuthMode::OAuth => "oauth",
            scotty_core::settings::api_server::AuthMode::Bearer => "bearer",
        },
    });
    Json(json_response)
}
