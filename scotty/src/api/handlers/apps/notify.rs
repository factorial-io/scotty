use axum::{extract::State, response::IntoResponse, Json};
use scotty_core::{
    apps::app_data::AppData,
    notification_types::{AddNotificationRequest, NotificationReceiver, RemoveNotificationRequest},
};

use crate::{api::error::AppError, app_state::SharedAppState};

#[utoipa::path(
    post,
    path = "/api/v1/apps/notify/add",
    request_body(content = AddNotificationRequest, content_type = "application/json"),
    responses(
    (status = 200, response = inline(AppData)),
    (status = 401, description = "Access token is missing or invalid"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn add_notification_handler(
    State(state): State<SharedAppState>,
    Json(payload): Json<AddNotificationRequest>,
) -> Result<impl IntoResponse, AppError> {
    let app_name = payload.app_name.clone();
    let service_ids = payload.service_ids.clone();

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

    Ok(Json(app))
}

#[utoipa::path(
    post,
    path = "/api/v1/apps/notify/remove",
    request_body(content = RemoveNotificationRequest, content_type = "application/json"),
    responses(
    (status = 200, response = inline(AppData)),
    (status = 401, description = "Access token is missing or invalid"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn remove_notification_handler(
    State(state): State<SharedAppState>,
    Json(payload): Json<RemoveNotificationRequest>,
) -> Result<impl IntoResponse, AppError> {
    let app_name = payload.app_name.clone();
    let service_ids = payload.service_ids.clone();

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

    Ok(Json(app))
}
