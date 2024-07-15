#![allow(dead_code)]

use axum::http::StatusCode;
use axum::{
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Service unavailable")]
    ServiceUnavailable,

    #[error("Not found")]
    NotFound,

    #[error("Internal server error")]
    InternalServerError(String),

    #[error("Invalid input")]
    InvalidInput,

    #[error("App not found")]
    AppNotFound(String),
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::InternalServerError(e.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body): (axum::http::StatusCode, String) = match self {
            AppError::ServiceUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Service unavailable".into(),
            ),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".into()),
            AppError::InternalServerError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal server error: {}", &msg),
            ),
            AppError::InvalidInput => (StatusCode::BAD_REQUEST, "Invalid input".into()),
            AppError::AppNotFound(app_id) => {
                (StatusCode::NOT_FOUND, format!("App not found: {}", app_id))
            }
        };
        let body = serde_json::json!({ "error": true, "message": body });
        (status, Json(body)).into_response()
    }
}
