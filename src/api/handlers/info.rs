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
    });
    Json(json_response)
}
