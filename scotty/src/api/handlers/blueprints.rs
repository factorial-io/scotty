use axum::{debug_handler, extract::State, response::IntoResponse, Json};
use scotty_core::settings::app_blueprint::AppBlueprintList;

use crate::app_state::SharedAppState;

#[debug_handler]
#[utoipa::path(
    get,
    path = "/api/v1/blueprints",
    responses(
    (status = 200, response = inline(AppBlueprintList))
    )
)]
pub async fn blueprints_handler(State(state): State<SharedAppState>) -> impl IntoResponse {
    let blueprints = AppBlueprintList {
        blueprints: state.settings.apps.blueprints.clone(),
    };

    Json(blueprints)
}
