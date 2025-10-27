use axum::{debug_handler, extract::State, response::IntoResponse, Json};
use subtle::ConstantTimeEq;
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

            // First, check if the provided token matches any configured bearer tokens
            // by doing a reverse lookup to find the identifier
            // Use constant-time comparison to prevent timing attacks
            let mut token_valid = false;
            for (identifier, configured_token) in &state.settings.api.bearer_tokens {
                if form
                    .password
                    .as_bytes()
                    .ct_eq(configured_token.as_bytes())
                    .into()
                {
                    // Found matching token, now check if this identifier has assignments
                    let auth_service = &state.auth_service;
                    let user_id = format!("identifier:{}", identifier);
                    if auth_service
                        .get_user_by_identifier(&user_id)
                        .await
                        .is_some()
                    {
                        debug!("Token validated for identifier: {}", identifier);
                        token_valid = true;
                        break;
                    }
                }
            }

            if token_valid {
                serde_json::json!({
                    "status": "success",
                    "auth_mode": "bearer",
                    "token": form.password.clone(),
                })
            } else {
                debug!("Token validation failed - token not found or no RBAC assignments");
                serde_json::json!({
                    "status": "error",
                    "auth_mode": "bearer",
                    "message": "Invalid token",
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
