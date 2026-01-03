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
    settings::{app_blueprint::ActionName, custom_action::CustomAction},
    tasks::running_app_context::RunningAppContext,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CustomActionPayload {
    pub action_name: String,
}

/// Result of looking up an action - either a per-app custom action or a blueprint action
enum ActionLookupResult<'a> {
    /// Per-app custom action (requires approval validation)
    PerAppAction(&'a CustomAction),
    /// Blueprint action (always executable)
    BlueprintAction(Permission),
}

#[debug_handler]
#[utoipa::path(
    post,
    path = "/api/v1/authenticated/apps/{app_name}/actions",
    request_body = CustomActionPayload,
    responses(
        (status = 200, response = inline(RunningAppContext)),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Insufficient permissions for this action or action not approved"),
        (status = 404, description = "App or action not found"),
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

    // Look up the action - check per-app custom actions first, then blueprint actions
    let action_name = ActionName::Custom(payload.action_name.clone());
    let lookup_result = lookup_action(&state, &app, &payload.action_name)?;

    // Get the required permission and validate executability
    let required_permission = match &lookup_result {
        ActionLookupResult::PerAppAction(custom_action) => {
            // CRITICAL: Validate that per-app custom actions are approved before execution
            if !custom_action.can_execute() {
                warn!(
                    "Action '{}' on app '{}' is not executable (status: {})",
                    payload.action_name, app_name, custom_action.status
                );
                return Err(AppError::ActionNotExecutable(format!(
                    "Action '{}' cannot be executed (status: {}). Only approved actions can be executed.",
                    payload.action_name, custom_action.status
                )));
            }
            custom_action.permission
        }
        ActionLookupResult::BlueprintAction(permission) => {
            // Blueprint actions are always executable (no approval workflow)
            *permission
        }
    };

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

/// Look up an action by name, checking per-app custom actions first, then blueprint actions.
///
/// This function implements the action resolution priority:
/// 1. Per-app custom actions (stored in AppSettings.custom_actions) - require approval
/// 2. Blueprint actions (defined in the app's blueprint) - always executable
fn lookup_action<'a>(
    state: &'a SharedAppState,
    app: &'a scotty_core::apps::app_data::AppData,
    action_name: &str,
) -> Result<ActionLookupResult<'a>, AppError> {
    // Get app settings
    let app_settings = app
        .settings
        .as_ref()
        .ok_or_else(|| AppError::AppSettingsNotFound(app.name.to_string()))?;

    // First, check per-app custom actions
    if let Some(custom_action) = app_settings.get_custom_action(action_name) {
        return Ok(ActionLookupResult::PerAppAction(custom_action));
    }

    // Fall back to blueprint actions
    let blueprint_name = app_settings.app_blueprint.as_ref().ok_or_else(|| {
        AppError::ActionNotFound(format!(
            "Action '{}' not found: app has no custom actions and no blueprint",
            action_name
        ))
    })?;

    let blueprint = state
        .settings
        .apps
        .blueprints
        .get(blueprint_name)
        .ok_or_else(|| AppError::AppBlueprintNotFound(blueprint_name.clone()))?;

    let action_key = ActionName::Custom(action_name.to_string());
    let blueprint_action = blueprint.actions.get(&action_key).ok_or_else(|| {
        AppError::ActionNotFound(format!(
            "Action '{}' not found in app custom actions or blueprint '{}'",
            action_name, blueprint_name
        ))
    })?;

    Ok(ActionLookupResult::BlueprintAction(
        blueprint_action.permission,
    ))
}
