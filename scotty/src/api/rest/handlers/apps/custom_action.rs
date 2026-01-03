use axum::{
    debug_handler,
    extract::{Path, State},
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;
use tracing::{info, warn};
use utoipa::ToSchema;

use crate::{
    api::{error::AppError, middleware::authorization::AuthorizationContext},
    app_state::SharedAppState,
    docker::run_app_custom_action::run_app_custom_action,
    services::AuthorizationService,
};
use scotty_core::{
    authorization::Permission,
    settings::app_blueprint::ActionName,
    tasks::running_app_context::RunningAppContext,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CustomActionPayload {
    pub action_name: String,
}

#[debug_handler]
#[utoipa::path(
    post,
    path = "/api/v1/authenticated/apps/{app_name}/actions",
    request_body = CustomActionPayload,
    responses(
        (status = 200, response = inline(RunningAppContext)),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Insufficient permissions for this action"),
        (status = 404, description = "App not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn run_custom_action_handler(
    State(state): State<SharedAppState>,
    Extension(auth_context): Extension<AuthorizationContext>,
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
        None => return Err(AppError::AppNotFound(app_name.clone())),
    };

    // Get the action's required permission from the blueprint
    let action_name = ActionName::Custom(payload.action_name.clone());
    let required_permission = get_action_permission(&state, &app, &action_name)?;

    // Check if user has the required permission for this action
    let user_id = AuthorizationService::get_user_id_for_authorization(&auth_context.user);
    let has_permission = state
        .auth_service
        .check_permission(&user_id, &app_name, &required_permission)
        .await;

    if !has_permission {
        warn!(
            "Access denied: user {} lacks {} permission for action '{}' on app '{}'",
            auth_context.user.email,
            required_permission.as_str(),
            payload.action_name,
            app_name
        );
        return Err(AppError::ScopeAccessDenied(format!(
            "Insufficient permission ({}) to run action '{}'",
            required_permission.as_str(),
            payload.action_name
        )));
    }

    info!(
        "Access granted: user {} has {} permission for action '{}' on app '{}'",
        auth_context.user.email,
        required_permission.as_str(),
        payload.action_name,
        app_name
    );

    // Create a task for running the custom action
    let app_data = run_app_custom_action(state, &app, action_name).await?;

    Ok(Json(app_data))
}

/// Get the required permission for an action from its definition
fn get_action_permission(
    state: &SharedAppState,
    app: &scotty_core::apps::app_data::AppData,
    action_name: &ActionName,
) -> Result<Permission, AppError> {
    // Get app settings
    let app_settings = app
        .settings
        .as_ref()
        .ok_or_else(|| AppError::AppSettingsNotFound(app.name.to_string()))?;

    // Get blueprint name
    let blueprint_name = app_settings
        .app_blueprint
        .as_ref()
        .ok_or_else(|| AppError::AppBlueprintMismatch("App doesn't have a blueprint".to_string()))?;

    // Get blueprint
    let blueprint = state
        .settings
        .apps
        .blueprints
        .get(blueprint_name)
        .ok_or_else(|| AppError::AppBlueprintNotFound(blueprint_name.clone()))?;

    // Get action
    let action = blueprint
        .actions
        .get(action_name)
        .ok_or_else(|| AppError::ActionNotFound(format!("Action not found: {action_name:?}")))?;

    // Return the action's permission (defaults to ActionWrite if not specified)
    Ok(action.permission)
}
