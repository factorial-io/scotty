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
    docker::{
        destroy_app::destroy_app, find_apps::inspect_app, purge_app::purge_app,
        rebuild_app::rebuild_app, run_app::run_app, stop_app::stop_app,
    },
    tasks::running_app_context::RunningAppContext,
};

#[utoipa::path(
    get,
    path = "/api/v1/apps/run/{app_id}",
    responses(
    (status = 200, response = inline(RunningAppContext))
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
    (status = 200, response = inline(RunningAppContext))
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
    path = "/api/v1/apps/purge/{app_id}",
    responses(
    (status = 200, response = inline(RunningAppContext))
    )
)]
#[debug_handler]
pub async fn purge_app_handler(
    Path(app_id): Path<String>,
    State(state): State<SharedAppState>,
) -> Result<impl IntoResponse, AppError> {
    let app_data = state.apps.get_app(&app_id).await;
    if app_data.is_none() {
        return Err(AppError::AppNotFound(app_id.clone()));
    }
    let app_data = app_data.unwrap();
    let app_data = purge_app(state, &app_data).await?;
    Ok(Json(app_data))
}

#[utoipa::path(
    get,
    path = "/api/v1/apps/info/{app_id}",
    responses(
    (status = 200, response = inline(AppData))
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

#[utoipa::path(
    get,
    path = "/api/v1/apps/rebuild/{app_id}",
    responses(
    (status = 200, response = inline(RunningAppContext))
    )
)]
#[debug_handler]
pub async fn rebuild_app_handler(
    Path(app_id): Path<String>,
    State(state): State<SharedAppState>,
) -> Result<impl IntoResponse, AppError> {
    let app_data = state.apps.get_app(&app_id).await;
    if app_data.is_none() {
        return Err(AppError::AppNotFound(app_id.clone()));
    }
    let app_data = app_data.unwrap();
    let app_data = rebuild_app(state, &app_data).await?;
    Ok(Json(app_data))
}

#[utoipa::path(
    get,
    path = "/api/v1/apps/destroy/{app_id}",
    responses(
    (status = 200, response = inline(RunningAppContext)),
    (status = 400, response = inline(AppError))
    )
)]
#[debug_handler]
pub async fn destroy_app_handler(
    Path(app_id): Path<String>,
    State(state): State<SharedAppState>,
) -> Result<impl IntoResponse, AppError> {
    let app_data = state.apps.get_app(&app_id).await;
    if app_data.is_none() {
        return Err(AppError::AppNotFound(app_id.clone()));
    }
    let app_data = app_data.unwrap();
    if app_data.settings.is_none() {
        return Err(AppError::CantDestroyUnmanagedApp(app_id.clone()));
    }
    let app_data = destroy_app(state, &app_data).await?;
    Ok(Json(app_data))
}

#[utoipa::path(
    get,
    path = "/api/v1/apps/migrate/{app_id}",
    responses(
    (status = 200, response = inline(AppData)),
    (status = 400, response = inline(AppError))
    )
)]
#[debug_handler]
pub async fn migrate_app_handler(
    Path(app_id): Path<String>,
    State(state): State<SharedAppState>,
) -> Result<impl IntoResponse, AppError> {
    let app_data = state.apps.get_app(&app_id).await;
    if app_data.is_none() {
        return Err(AppError::AppNotFound(app_id.clone()));
    }
    let app_data = app_data.unwrap();
    let docker_compose_path = std::path::PathBuf::from(app_data.docker_compose_path.clone());
    let app_data = inspect_app(&state, &docker_compose_path).await?;

    if app_data.settings.is_some() {
        return Err(AppError::CantMigrateAppWithExistingSettings(app_id.clone()));
    }
    let app_data = app_data.create_settings_from_runtime().await?;
    state.apps.update_app(app_data.clone()).await?;

    Ok(Json(app_data))
}
