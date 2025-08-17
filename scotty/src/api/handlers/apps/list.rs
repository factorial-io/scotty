use axum::{debug_handler, extract::State, response::IntoResponse};
use scotty_core::apps::shared_app_list::AppDataVec;

use crate::{api::error::AppError, api::secure_response::SecureJson, app_state::SharedAppState};
#[utoipa::path(
    get,
    path = "/api/v1/authenticated/apps/list",
    responses(
    (status = 200, response = inline(AppDataVec)),
    (status = 401, description = "Access token is missing or invalid"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
#[debug_handler]
pub async fn list_apps_handler(
    State(state): State<SharedAppState>,
) -> Result<impl IntoResponse, AppError> {
    let apps = state.apps.get_apps().await;
    Ok(SecureJson(apps))
}
