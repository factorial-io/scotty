use axum::{debug_handler, extract::State, response::IntoResponse, Json};

use crate::app_state::SharedAppState;

#[derive(serde::Deserialize)]
pub struct FormData {
    pub password: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/login",
    responses(
    (status = 200, description = "Login via bearer token")
    )
)]
#[debug_handler]
pub async fn login_handler(
    State(state): State<SharedAppState>,
    Json(form): Json<FormData>,
) -> impl IntoResponse {
    let access_token = state.settings.api.access_token.as_ref();

    let json_response = if access_token.is_some() && &form.password != access_token.unwrap() {
        serde_json::json!({
            "status": "error",
            "message": "Invalid token",
        })
    } else {
        serde_json::json!({
            "status": "success",
            "token": form.password.clone(),
        })
    };

    Json(json_response)
}

#[utoipa::path(
    post,
    path = "/api/v1/validate-token",
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
