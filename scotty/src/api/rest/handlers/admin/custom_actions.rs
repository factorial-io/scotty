use axum::{
    debug_handler,
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::info;
use utoipa::ToSchema;

use crate::{api::error::AppError, app_state::SharedAppState};
use scotty_core::settings::custom_action::{ActionStatus, CustomAction, ReviewActionRequest};

/// Pending action with app context
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PendingActionInfo {
    pub app_name: String,
    pub action: CustomAction,
}

/// Response for listing pending actions
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PendingActionsResponse {
    pub pending_actions: Vec<PendingActionInfo>,
}

/// List all pending custom actions across all apps
#[debug_handler]
#[utoipa::path(
    get,
    path = "/api/v1/authenticated/admin/actions/pending",
    responses(
        (status = 200, description = "List of pending actions", body = PendingActionsResponse),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Insufficient permissions"),
        (status = 500, description = "Internal server error"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn list_pending_actions_handler(
    State(state): State<SharedAppState>,
) -> Result<impl IntoResponse, AppError> {
    info!("Listing all pending custom actions");

    let apps = state.apps.get_apps().await;
    let mut pending_actions = Vec::new();

    for app in apps.apps.iter() {
        if let Some(settings) = &app.settings {
            for action in settings.custom_actions.values() {
                if action.status == ActionStatus::Pending {
                    pending_actions.push(PendingActionInfo {
                        app_name: app.name.clone(),
                        action: action.clone(),
                    });
                }
            }
        }
    }

    Ok(Json(PendingActionsResponse { pending_actions }))
}

/// Get details of a specific custom action (admin view)
#[debug_handler]
#[utoipa::path(
    get,
    path = "/api/v1/authenticated/admin/actions/{app_name}/{action_name}",
    responses(
        (status = 200, description = "Custom action details", body = CustomAction),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "App or action not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn get_action_details_handler(
    State(state): State<SharedAppState>,
    Path((app_name, action_name)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        "Getting action details for '{}' in app '{}'",
        action_name, app_name
    );

    let app = match state.apps.get_app(&app_name).await {
        Some(app) => app,
        None => return Err(AppError::AppNotFound(app_name)),
    };

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

/// Approve a pending custom action
#[debug_handler]
#[utoipa::path(
    post,
    path = "/api/v1/authenticated/admin/actions/{app_name}/{action_name}/approve",
    request_body = ReviewActionRequest,
    responses(
        (status = 200, description = "Action approved", body = CustomAction),
        (status = 400, description = "Action is not in pending state"),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "App or action not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn approve_action_handler(
    State(state): State<SharedAppState>,
    Path((app_name, action_name)): Path<(String, String)>,
    axum::Extension(user_id): axum::Extension<String>,
    Json(payload): Json<ReviewActionRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        "Approving action '{}' in app '{}' by '{}'",
        action_name, app_name, user_id
    );

    update_action_status(
        &state,
        &app_name,
        &action_name,
        &user_id,
        payload.comment,
        ReviewOperation::Approve,
    )
    .await
}

/// Reject a pending custom action
#[debug_handler]
#[utoipa::path(
    post,
    path = "/api/v1/authenticated/admin/actions/{app_name}/{action_name}/reject",
    request_body = ReviewActionRequest,
    responses(
        (status = 200, description = "Action rejected", body = CustomAction),
        (status = 400, description = "Action is not in pending state"),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "App or action not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn reject_action_handler(
    State(state): State<SharedAppState>,
    Path((app_name, action_name)): Path<(String, String)>,
    axum::Extension(user_id): axum::Extension<String>,
    Json(payload): Json<ReviewActionRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        "Rejecting action '{}' in app '{}' by '{}'",
        action_name, app_name, user_id
    );

    update_action_status(
        &state,
        &app_name,
        &action_name,
        &user_id,
        payload.comment,
        ReviewOperation::Reject,
    )
    .await
}

/// Revoke a previously approved custom action
#[debug_handler]
#[utoipa::path(
    post,
    path = "/api/v1/authenticated/admin/actions/{app_name}/{action_name}/revoke",
    request_body = ReviewActionRequest,
    responses(
        (status = 200, description = "Action revoked", body = CustomAction),
        (status = 400, description = "Action is not in approved state"),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "App or action not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn revoke_action_handler(
    State(state): State<SharedAppState>,
    Path((app_name, action_name)): Path<(String, String)>,
    axum::Extension(user_id): axum::Extension<String>,
    Json(payload): Json<ReviewActionRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        "Revoking action '{}' in app '{}' by '{}'",
        action_name, app_name, user_id
    );

    update_action_status(
        &state,
        &app_name,
        &action_name,
        &user_id,
        payload.comment,
        ReviewOperation::Revoke,
    )
    .await
}

/// Review operation type
enum ReviewOperation {
    Approve,
    Reject,
    Revoke,
}

impl ReviewOperation {
    fn required_status(&self) -> ActionStatus {
        match self {
            ReviewOperation::Approve | ReviewOperation::Reject => ActionStatus::Pending,
            ReviewOperation::Revoke => ActionStatus::Approved,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            ReviewOperation::Approve => "approve",
            ReviewOperation::Reject => "reject",
            ReviewOperation::Revoke => "revoke",
        }
    }

    fn apply(&self, action: &mut CustomAction, reviewer: String, comment: Option<String>) {
        match self {
            ReviewOperation::Approve => action.approve(reviewer, comment),
            ReviewOperation::Reject => action.reject(reviewer, comment),
            ReviewOperation::Revoke => action.revoke(reviewer, comment),
        }
    }
}

/// Helper function to update action status
async fn update_action_status(
    state: &SharedAppState,
    app_name: &str,
    action_name: &str,
    user_id: &str,
    comment: Option<String>,
    operation: ReviewOperation,
) -> Result<impl IntoResponse, AppError> {
    let app = match state.apps.get_app(app_name).await {
        Some(app) => app,
        None => return Err(AppError::AppNotFound(app_name.to_string())),
    };

    let mut settings = app.settings.clone().unwrap_or_default();
    let action = settings.get_custom_action_mut(action_name);

    match action {
        Some(action) => {
            let required_status = operation.required_status();
            if action.status != required_status {
                return Err(AppError::ActionNotFound(format!(
                    "Cannot {} action '{}': action is in '{}' state, expected '{}'",
                    operation.name(),
                    action_name,
                    action.status.as_str(),
                    required_status.as_str()
                )));
            }

            operation.apply(action, user_id.to_string(), comment);
            let updated_action = action.clone();

            // Save the updated settings
            let updated_app = scotty_core::apps::app_data::AppData {
                settings: Some(settings),
                ..app.clone()
            };
            updated_app.save_settings().await?;
            state.apps.update_app(updated_app).await?;

            Ok(Json(updated_action))
        }
        None => Err(AppError::ActionNotFound(format!(
            "Custom action '{}' not found in app '{}'",
            action_name, app_name
        ))),
    }
}
