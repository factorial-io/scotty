use axum::{
    debug_handler,
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use tracing::info;
use utoipa::ToSchema;

use crate::{
    api::error::AppError, app_state::SharedAppState,
    docker::run_app_custom_action::run_app_custom_action,
};
use scotty_core::{
    settings::app_blueprint::ActionName, tasks::running_app_context::RunningAppContext,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CustomActionPayload {
    pub action_name: String,
}

#[debug_handler]
#[utoipa::path(
    post,
    path = "/api/v1/apps/{app_name}/actions",
    request_body = CustomActionPayload,
    responses(
        (status = 200, response = inline(RunningAppContext)),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 404, description = "App not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn run_custom_action_handler(
    State(state): State<SharedAppState>,
    Path(app_name): Path<String>,
    Json(payload): Json<CustomActionPayload>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        "Starting custom action '{}' for app '{}'",
        payload.action_name, app_name
    );

    // Validate app exists and get the app
    let app = match state.apps.get_app(&app_name).await {
        Some(app) => app,
        None => return Err(AppError::AppNotFound(app_name)),
    };

    // Create a task for running the custom action
    let app_data =
        run_app_custom_action(state, &app, ActionName::Custom(payload.action_name)).await?;

    Ok(Json(app_data))
}
