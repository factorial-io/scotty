#![allow(dead_code)]

use axum::http::StatusCode;
use axum::{
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Error, Debug, utoipa::ToResponse)]
pub enum AppError {
    #[error("Service unavailable")]
    ServiceUnavailable,

    #[error("Not found")]
    NotFound,

    #[error("Internal server error: {0}")]
    InternalServerError(String),

    #[error("Invalid input")]
    InvalidInput,

    #[error("App not found: {0}")]
    AppNotFound(String),

    #[error("Task not found: {0}")]
    TaskNotFound(Uuid),

    #[error("File content could not be decoded!")]
    FileContentDecodingError,

    #[error("Cant destroy an unmanaged app: {0}")]
    CantDestroyUnmanagedApp(String),

    #[error("Missing docker-compose file in the payload")]
    NoDockerComposeFile,

    #[error("Invalid docker-compose file")]
    InvalidDockerComposeFile,

    #[error("Service not found in docker compose file: {0}")]
    PublicServiceNotFound(String),

    #[error("Public ports for service {0} are not supported")]
    PublicPortsNotSupported(String),

    #[error("Environment variables for variable substitution like {0} are not supported")]
    EnvironmentVariablesNotSupported(String),

    #[error("Operation not supported for legacy/ unsupported app {0}")]
    OperationNotSupportedForLegacyApp(String),

    #[error("Private registry not found in settings: {0}")]
    RegistryNotFound(String),

    #[error("App blueprint not found in settings: {0}")]
    AppBlueprintNotFound(String),

    #[error("App blueprint mismatch: {0}")]
    AppBlueprintMismatch(String),

    #[error("App settings not found for app: {0}")]
    AppSettingsNotFound(String),

    #[error("Found invalid notification service ids: {0}")]
    InvalidNotificationServiceIds(String),

    #[error(
        "Cant migrate app {0} with existing settings, app can already be controlled by scotty!"
    )]
    CantMigrateAppWithExistingSettings(String),
}
impl AppError {
    fn get_error_msg(&self) -> (axum::http::StatusCode, String) {
        let status: axum::http::StatusCode = match self {
            AppError::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::AppNotFound(_) => StatusCode::NOT_FOUND,
            AppError::TaskNotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, self.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        if let Some(app_error) = e.downcast_ref::<AppError>() {
            return app_error.clone();
        }
        AppError::InternalServerError(e.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = self.get_error_msg();
        let body = serde_json::json!({ "error": true, "message": body });
        (status, Json(body)).into_response()
    }
}
