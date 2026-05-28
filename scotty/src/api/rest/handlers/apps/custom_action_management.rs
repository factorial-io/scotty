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

/// Maximum length allowed for a custom action name.
const MAX_ACTION_NAME_LEN: usize = 255;

/// Validate and normalize a custom action name.
///
/// The name is used as a `HashMap` key and is embedded directly into URL path
/// segments (e.g. `/apps/{app}/custom-actions/{name}` and the admin
/// approve/reject/revoke routes), so it must be non-empty, bounded in length,
/// and restricted to URL-safe characters. A `/`, `?`, `#` or `%` would split or
/// corrupt the path and make the action unreachable. Surrounding whitespace is
/// trimmed so the stored key is clean.
fn validate_action_name(raw: &str) -> Result<String, AppError> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest(
            "Custom action name must not be empty".to_string(),
        ));
    }
    if name.len() > MAX_ACTION_NAME_LEN {
        return Err(AppError::BadRequest(format!(
            "Custom action name must be at most {MAX_ACTION_NAME_LEN} characters"
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
    {
        return Err(AppError::BadRequest(format!(
            "Invalid custom action name '{name}'. Use only letters, digits, and the characters '_', '-', '.', ':'"
        )));
    }
    Ok(name.to_string())
}

/// Create a new custom action for an app
#[debug_handler]
#[utoipa::path(
    post,
    path = "/api/v1/authenticated/apps/{app_name}/custom-actions",
    request_body = CreateCustomActionRequest,
    responses(
        (status = 201, description = "Custom action created", body = CustomAction),
        (status = 400, description = "Invalid request (e.g. bad name or permission)"),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 404, description = "App not found"),
        (status = 409, description = "An action with this name already exists"),
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

    let name = validate_action_name(&payload.name)?;

    // Parse permission string and restrict it to the two permissions that
    // gate action *execution*. Management/approval permissions
    // (`action_manage`, `action_approve`) must not be usable as the bar for
    // running an action, otherwise creators could raise or lower the gate to
    // an unintended level.
    let permission = Permission::from_str(&payload.permission).ok_or_else(|| {
        AppError::BadRequest(format!(
            "Invalid permission '{}'. Use 'action_read' or 'action_write'",
            payload.permission
        ))
    })?;
    if !matches!(permission, Permission::ActionRead | Permission::ActionWrite) {
        return Err(AppError::BadRequest(format!(
            "Invalid permission '{}'. Use 'action_read' or 'action_write'",
            payload.permission
        )));
    }

    // Create the custom action
    let action = CustomAction::new(
        name,
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

    let mut actions: Vec<CustomAction> = app
        .settings
        .as_ref()
        .map(|s| s.custom_actions.values().cloned().collect())
        .unwrap_or_default();

    // `custom_actions` is a HashMap, so sort by name for deterministic output.
    actions.sort_by(|a, b| a.name.cmp(&b.name));

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_action_name_trims_and_accepts_safe_names() {
        assert_eq!(validate_action_name("  deploy-db  ").unwrap(), "deploy-db");
        assert_eq!(
            validate_action_name("web:migrate.v2").unwrap(),
            "web:migrate.v2"
        );
        assert_eq!(validate_action_name("Action_1").unwrap(), "Action_1");
    }

    #[test]
    fn validate_action_name_rejects_empty() {
        assert!(matches!(
            validate_action_name("   "),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn validate_action_name_rejects_too_long() {
        let long = "a".repeat(MAX_ACTION_NAME_LEN + 1);
        assert!(matches!(
            validate_action_name(&long),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn validate_action_name_rejects_url_unsafe_chars() {
        for bad in ["my/action", "a?b", "a#b", "a%2f", "with space", "emoji😀"] {
            assert!(
                matches!(validate_action_name(bad), Err(AppError::BadRequest(_))),
                "expected '{bad}' to be rejected"
            );
        }
    }
}
