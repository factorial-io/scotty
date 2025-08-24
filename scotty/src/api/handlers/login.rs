use axum::{debug_handler, extract::State, response::IntoResponse, Json};
use tracing::debug;

use crate::app_state::SharedAppState;
use scotty_core::settings::api_server::AuthMode;

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct FormData {
    pub password: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/login",
    responses(
    (status = 200, description = "Login endpoint - behavior depends on auth mode")
    )
)]
#[debug_handler]
pub async fn login_handler(
    State(state): State<SharedAppState>,
    Json(form): Json<FormData>,
) -> impl IntoResponse {
    debug!(
        "Login attempt with auth mode: {:?}",
        state.settings.api.auth_mode
    );

    let json_response = match state.settings.api.auth_mode {
        AuthMode::Development => {
            debug!("Development mode login - always successful");
            serde_json::json!({
                "status": "success",
                "auth_mode": "dev",
                "message": "Development mode - login not required"
            })
        }
        AuthMode::OAuth => {
            debug!("OAuth mode login - redirect to native OAuth flow");
            serde_json::json!({
                "status": "redirect",
                "auth_mode": "oauth",
                "redirect_url": "/oauth/authorize",
                "message": "Please authenticate via GitLab OAuth"
            })
        }
        AuthMode::Bearer => {
            debug!("Bearer token login attempt");
            let access_token = state.settings.api.access_token.as_ref();

            if access_token.is_some() && &form.password != access_token.unwrap() {
                serde_json::json!({
                    "status": "error",
                    "auth_mode": "bearer",
                    "message": "Invalid token",
                })
            } else {
                serde_json::json!({
                    "status": "success",
                    "auth_mode": "bearer",
                    "token": form.password.clone(),
                })
            }
        }
    };

    Json(json_response)
}

#[utoipa::path(
    post,
    path = "/api/v1/authenticated/validate-token",
    responses(
    (status = 200, description = "Validate token")
    )
)]
pub async fn validate_token_handler() -> impl IntoResponse {
    let json_response = serde_json::json!({
        "status": "success",
    });

    Json(json_response)
}
