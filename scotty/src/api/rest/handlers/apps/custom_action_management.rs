use axum::{
    debug_handler,
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use tracing::info;

use crate::{api::error::AppError, app_state::SharedAppState};
use scotty_core::authorization::Permission;
use scotty_core::settings::custom_action::{
    CreateCustomActionRequest, CustomAction, CustomActionList,
};

/// Create a new custom action for an app
#[debug_handler]
#[utoipa::path(
    post,
    path = "/api/v1/authenticated/apps/{app_name}/custom-actions",
    request_body = CreateCustomActionRequest,
    responses(
        (status = 201, description = "Custom action created", body = CustomAction),
        (status = 400, description = "Invalid request or action already exists"),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 404, description = "App not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn create_custom_action_handler(
    State(state): State<SharedAppState>,
    Path(app_name): Path<String>,
    axum::Extension(user_id): axum::Extension<String>,
    Json(payload): Json<CreateCustomActionRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        "Creating custom action '{}' for app '{}' by user '{}'",
        payload.name, app_name, user_id
    );

    // Validate app exists
    let app = match state.apps.get_app(&app_name).await {
        Some(app) => app,
        None => return Err(AppError::AppNotFound(app_name)),
    };

    // Parse permission string
    let permission = Permission::from_str(&payload.permission).ok_or_else(|| {
        AppError::BadRequest(format!(
            "Invalid permission '{}'. Use 'action_read' or 'action_write'",
            payload.permission
        ))
    })?;

    // Create the custom action
    let action = CustomAction::new(
        payload.name.clone(),
        payload.description,
        payload.commands,
        permission,
        user_id,
    );

    // Get or create settings and add the action
    let mut settings = app.settings.clone().unwrap_or_default();
    settings
        .add_custom_action(action.clone())
        .map_err(AppError::ActionAlreadyExists)?;

    // Update app with new settings
    let updated_app = scotty_core::apps::app_data::AppData {
        settings: Some(settings),
        ..app.clone()
    };
    updated_app.save_settings().await?;
    state.apps.update_app(updated_app).await?;

    Ok((axum::http::StatusCode::CREATED, Json(action)))
}

/// List all custom actions for an app
#[debug_handler]
#[utoipa::path(
    get,
    path = "/api/v1/authenticated/apps/{app_name}/custom-actions",
    responses(
        (status = 200, description = "List of custom actions", body = CustomActionList),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 404, description = "App not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn list_custom_actions_handler(
    State(state): State<SharedAppState>,
    Path(app_name): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    info!("Listing custom actions for app '{}'", app_name);

    // Validate app exists
    let app = match state.apps.get_app(&app_name).await {
        Some(app) => app,
        None => return Err(AppError::AppNotFound(app_name)),
    };

    let actions: Vec<CustomAction> = app
        .settings
        .as_ref()
        .map(|s| s.custom_actions.values().cloned().collect())
        .unwrap_or_default();

    Ok(Json(CustomActionList { actions }))
}

/// Delete a custom action from an app
#[debug_handler]
#[utoipa::path(
    delete,
    path = "/api/v1/authenticated/apps/{app_name}/custom-actions/{action_name}",
    responses(
        (status = 200, description = "Custom action deleted", body = CustomAction),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 404, description = "App or action not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn delete_custom_action_handler(
    State(state): State<SharedAppState>,
    Path((app_name, action_name)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        "Deleting custom action '{}' from app '{}'",
        action_name, app_name
    );

    // Validate app exists
    let app = match state.apps.get_app(&app_name).await {
        Some(app) => app,
        None => return Err(AppError::AppNotFound(app_name)),
    };

    // Get settings and remove the action
    let mut settings = app.settings.clone().unwrap_or_default();
    let removed = settings.remove_custom_action(&action_name);

    match removed {
        Some(action) => {
            // Update app with new settings
            let updated_app = scotty_core::apps::app_data::AppData {
                settings: Some(settings),
                ..app.clone()
            };
            updated_app.save_settings().await?;
            state.apps.update_app(updated_app).await?;

            Ok(Json(action))
        }
        None => Err(AppError::ActionNotFound(format!(
            "Custom action '{}' not found in app '{}'",
            action_name, app_name
        ))),
    }
}

/// Get details of a specific custom action
#[debug_handler]
#[utoipa::path(
    get,
    path = "/api/v1/authenticated/apps/{app_name}/custom-actions/{action_name}",
    responses(
        (status = 200, description = "Custom action details", body = CustomAction),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 404, description = "App or action not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn get_custom_action_handler(
    State(state): State<SharedAppState>,
    Path((app_name, action_name)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        "Getting custom action '{}' from app '{}'",
        action_name, app_name
    );

    // Validate app exists
    let app = match state.apps.get_app(&app_name).await {
        Some(app) => app,
        None => return Err(AppError::AppNotFound(app_name)),
    };

    // Get the action
    let action = app
        .settings
        .as_ref()
        .and_then(|s| s.get_custom_action(&action_name))
        .cloned();

    match action {
        Some(action) => Ok(Json(action)),
        None => Err(AppError::ActionNotFound(format!(
            "Custom action '{}' not found in app '{}'",
            action_name, app_name
        ))),
    }
}
