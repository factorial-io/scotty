use axum::{extract::State, response::IntoResponse, Extension, Json};
use scotty_core::{
    apps::app_data::AppData,
    authorization::Permission,
    notification_types::{AddNotificationRequest, NotificationReceiver, RemoveNotificationRequest},
};

use crate::{
    api::{
        error::AppError, middleware::authorization::AuthorizationContext,
        secure_response::SecureJson,
    },
    app_state::SharedAppState,
    services::{authorization::decision::Target, AuthorizationService},
};

/// Changing an app's notifications requires `manage` on that app.
///
/// The app name comes from the request body, so the route-level permission
/// middleware cannot check it. The check runs before the app lookup so callers
/// cannot probe which apps exist in scopes they have no access to.
async fn require_manage(
    state: &SharedAppState,
    auth_context: &AuthorizationContext,
    app_name: &str,
) -> Result<(), AppError> {
    let user_id = AuthorizationService::get_user_id_for_authorization(&auth_context.user);
    state
        .auth_service
        .require(&user_id, Permission::Manage.into(), Target::App(app_name))
}

#[utoipa::path(
    post,
    path = "/api/v1/authenticated/apps/notify/add",
    request_body(content = AddNotificationRequest, content_type = "application/json"),
    responses(
    (status = 200, response = inline(AppData)),
    (status = 401, description = "Access token is missing or invalid"),
    (status = 403, description = "Caller lacks manage permission for the app"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn add_notification_handler(
    State(state): State<SharedAppState>,
    Extension(auth_context): Extension<AuthorizationContext>,
    Json(payload): Json<AddNotificationRequest>,
) -> Result<impl IntoResponse, AppError> {
    let app_name = payload.app_name.clone();
    let service_ids = payload.service_ids.clone();

    require_manage(&state, &auth_context, &app_name).await?;

    let app = state
        .apps
        .get_app(&app_name)
        .await
        .ok_or_else(|| AppError::AppNotFound(app_name.clone()))?;

    if app.settings.is_none() {
        return Err(AppError::AppSettingsNotFound(app_name));
    }

    let service_settings = &state.settings;
    let invalid_service_ids: Vec<_> = service_ids
        .iter()
        .filter(|id| !service_settings.notification_services.contains(id))
        .collect();

    if !invalid_service_ids.is_empty() {
        let invalid_service_ids = invalid_service_ids
            .into_iter()
            .map(|service_id| match service_id {
                NotificationReceiver::Log => "log",
                NotificationReceiver::Mattermost(id) => &id.service_id,
                NotificationReceiver::Gitlab(id) => &id.service_id,
                NotificationReceiver::Webhook(id) => &id.service_id,
            })
            .collect::<Vec<_>>();
        return Err(AppError::InvalidNotificationServiceIds(
            invalid_service_ids.join(", "),
        ));
    }

    let app = app.add_notifications(&service_ids);
    app.save_settings().await?;
    state.apps.update_app(app.clone()).await?;

    Ok(SecureJson(app))
}

#[utoipa::path(
    post,
    path = "/api/v1/authenticated/apps/notify/remove",
    request_body(content = RemoveNotificationRequest, content_type = "application/json"),
    responses(
    (status = 200, response = inline(AppData)),
    (status = 401, description = "Access token is missing or invalid"),
    (status = 403, description = "Caller lacks manage permission for the app"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn remove_notification_handler(
    State(state): State<SharedAppState>,
    Extension(auth_context): Extension<AuthorizationContext>,
    Json(payload): Json<RemoveNotificationRequest>,
) -> Result<impl IntoResponse, AppError> {
    let app_name = payload.app_name.clone();
    let service_ids = payload.service_ids.clone();

    require_manage(&state, &auth_context, &app_name).await?;

    let app = state
        .apps
        .get_app(&app_name)
        .await
        .ok_or_else(|| AppError::AppNotFound(app_name.clone()))?;

    if app.settings.is_none() {
        return Err(AppError::AppSettingsNotFound(app_name));
    }

    let app = app.remove_notifications(&service_ids);
    app.save_settings().await?;
    state.apps.update_app(app.clone()).await?;

    Ok(SecureJson(app))
}
