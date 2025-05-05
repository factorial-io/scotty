use axum::{
    debug_handler,
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{api::error::AppError, app_state::SharedAppState};
use scotty_core::tasks::task_details::TaskDetails;

#[utoipa::path(
    get,
    path = "/api/v1/task/{uuid}",
    responses(
    (status = 200, response = inline(TaskDetails)),
    (status = 401, description = "Access token is missing or invalid"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn task_detail_handler(
    Path(uuid): Path<Uuid>,
    State(state): State<SharedAppState>,
) -> impl IntoResponse {
    let task_detail = state.task_manager.get_task_details(&uuid).await;
    if task_detail.is_none() {
        return Err(AppError::TaskNotFound(uuid));
    }
    let task_detail = task_detail.unwrap();
    Ok(Json(task_detail))
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToResponse, utoipa::ToSchema)]
pub struct TaskList {
    pub tasks: Vec<TaskDetails>,
}

#[utoipa::path(
    get,
    path = "/api/v1/tasks",
    responses(
    (status = 200, response = inline(TaskList)),
    (status = 401, description = "Access token is missing or invalid"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
#[debug_handler]
pub async fn task_list_handler(
    State(state): State<SharedAppState>,
) -> Result<Json<TaskList>, AppError> {
    let task_list = TaskList {
        tasks: state.task_manager.get_task_list().await,
    };
    let json = Json(task_list);
    Ok(json)
}
