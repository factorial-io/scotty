use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

use crate::{
    api::error::AppError,
    app_state::SharedAppState,
    docker::services::logs::LogStreamingService,
};
use scotty_core::utils::slugify::slugify;

#[derive(Deserialize, ToSchema, utoipa::IntoParams)]
pub struct LogsQuery {
    /// Whether to follow log stream (default: false)
    #[serde(default)]
    pub follow: bool,
    /// Number of lines to include from the end of logs (default: 100)
    #[serde(default = "default_tail")]
    pub tail: String,
}

fn default_tail() -> String {
    "100".to_string()
}

#[derive(Serialize, ToSchema, utoipa::ToResponse)]
pub struct LogsStreamResponse {
    pub stream_id: Uuid,
    pub message: String,
}

#[derive(Serialize, ToSchema, utoipa::ToResponse)]
pub struct StopLogsResponse {
    pub message: String,
}

/// Start streaming logs for a service
#[utoipa::path(
    post,
    path = "/api/v1/authenticated/apps/{app_id}/services/{service_name}/logs",
    params(
        ("app_id" = String, Path, description = "Application ID"),
        ("service_name" = String, Path, description = "Service name to stream logs from"),
        LogsQuery
    ),
    responses(
        (status = 200, response = inline(LogsStreamResponse)),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 404, description = "App or service not found"),
        (status = 409, description = "Service has no container ID")
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn start_logs_handler(
    Path((app_id, service_name)): Path<(String, String)>,
    Query(query): Query<LogsQuery>,
    State(state): State<SharedAppState>,
) -> Result<impl IntoResponse, AppError> {
    let app_id = slugify(&app_id);
    let app_data = state.apps.get_app(&app_id).await
        .ok_or_else(|| AppError::AppNotFound(app_id.clone()))?;

    let logs_service = LogStreamingService::new(state.docker.clone());

    let stream_id = logs_service.start_stream(
        &state,
        &app_data,
        &service_name,
        query.follow,
        Some(query.tail),
    ).await?;

    Ok(axum::Json(LogsStreamResponse {
        stream_id,
        message: format!("Started log stream for {}/{}", app_id, service_name),
    }))
}

/// Stop a log stream
#[utoipa::path(
    delete,
    path = "/api/v1/authenticated/logs/streams/{stream_id}",
    params(
        ("stream_id" = Uuid, Path, description = "Stream ID to stop")
    ),
    responses(
        (status = 200, response = inline(StopLogsResponse)),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 404, description = "Stream not found")
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn stop_logs_handler(
    Path(stream_id): Path<Uuid>,
    State(state): State<SharedAppState>,
) -> Result<impl IntoResponse, AppError> {
    let logs_service = LogStreamingService::new(state.docker.clone());

    logs_service.stop_stream(stream_id).await?;

    Ok(axum::Json(StopLogsResponse {
        message: format!("Stopped log stream {}", stream_id),
    }))
}