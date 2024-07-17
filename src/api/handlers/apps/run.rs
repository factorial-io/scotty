use axum::{
    debug_handler,
    extract::{Path, State},
    response::IntoResponse,
    Json,
};

use crate::{
    api::error::AppError,
    app_state::SharedAppState,
    apps::app_data::AppData,
    docker::start_stop_app::{rm_app, run_app, stop_app},
};

#[utoipa::path(
    get,
    path = "/api/v1/apps/run/{app_id}",
    responses(
    (status = 200, response = AppData)
    )
)]
#[debug_handler]
pub async fn run_app_handler(
    Path(app_id): Path<String>,
    State(state): State<SharedAppState>,
) -> Result<impl IntoResponse, AppError> {
    let app_data = state.apps.get_app(&app_id).await;
    if app_data.is_none() {
        return Err(AppError::AppNotFound(app_id.clone()));
    }
    let app_data = app_data.unwrap();
    let app_data = run_app(state, &app_data).await?;
    Ok(Json(app_data))
}

#[utoipa::path(
    get,
    path = "/api/v1/apps/stop/{app_id}",
    responses(
    (status = 200, response = AppData)
    )
)]
#[debug_handler]
pub async fn stop_app_handler(
    Path(app_id): Path<String>,
    State(state): State<SharedAppState>,
) -> Result<impl IntoResponse, AppError> {
    let app_data = state.apps.get_app(&app_id).await;
    if app_data.is_none() {
        return Err(AppError::AppNotFound(app_id.clone()));
    }
    let app_data = app_data.unwrap();
    let app_data = stop_app(state, &app_data).await?;
    Ok(Json(app_data))
}

#[utoipa::path(
    get,
    path = "/api/v1/apps/rm/{app_id}",
    responses(
    (status = 200, response = AppData)
    )
)]
#[debug_handler]
pub async fn rm_app_handler(
    Path(app_id): Path<String>,
    State(state): State<SharedAppState>,
) -> Result<impl IntoResponse, AppError> {
    let app_data = state.apps.get_app(&app_id).await;
    if app_data.is_none() {
        return Err(AppError::AppNotFound(app_id.clone()));
    }
    let app_data = app_data.unwrap();
    let app_data = rm_app(state, &app_data).await?;
    Ok(Json(app_data))
}

#[utoipa::path(
    get,
    path = "/api/v1/apps/info/{app_id}",
    responses(
    (status = 200, response = AppData)
    )
)]
#[debug_handler]
pub async fn info_app_handler(
    Path(app_id): Path<String>,
    State(state): State<SharedAppState>,
) -> Result<impl IntoResponse, AppError> {
    let app_data = state.apps.get_app(&app_id).await;
    if app_data.is_none() {
        return Err(AppError::AppNotFound(app_id.clone()));
    }
    let app_data = app_data.unwrap();
    Ok(Json(app_data))
}
