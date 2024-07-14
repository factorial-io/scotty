use axum::{debug_handler, extract::State, response::IntoResponse, Json};

use crate::{api::error::AppError, app_state::SharedAppState, apps::shared_app_list::AppDataVec};
#[utoipa::path(
    get,
    path = "/api/v1/apps/list",
    responses(
    (status = 200, response = AppDataVec)
    )
)]
#[debug_handler]
pub async fn list_apps_handler(
    State(state): State<SharedAppState>,
) -> Result<impl IntoResponse, AppError> {
    let apps = state.apps.get_apps().await;
    Ok(Json(apps))
}
