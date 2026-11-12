use axum::{debug_handler, extract::State, response::IntoResponse, Extension, Json};
use scotty_core::settings::app_blueprint::AppBlueprintList;

use crate::api::middleware::authorization::AuthorizationContext;
use crate::app_state::SharedAppState;
use crate::services::authorization::decision::Target;
use crate::services::{apps, AuthorizationService};

/// Lists the blueprints the user may create apps from. A templated
/// blueprint carries its contract (schema, images, payload); Scotty-filled
/// schema properties are marked `readOnly` so forms don't ask for them.
#[debug_handler]
#[utoipa::path(
    get,
    path = "/api/v1/authenticated/blueprints",
    responses(
    (status = 200, response = inline(AppBlueprintList)),
    (status = 401, description = "Access token is missing or invalid"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn blueprints_handler(
    State(state): State<SharedAppState>,
    Extension(auth_context): Extension<AuthorizationContext>,
) -> impl IntoResponse {
    let user = AuthorizationService::get_user_id_for_authorization(&auth_context.user);
    Json(AppBlueprintList {
        blueprints: apps::blueprints(&state, &user, Target::Global),
    })
}
