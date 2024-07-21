use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::{api::error::AppError, app_state::SharedAppState, tasks::manager::TaskDetails};

#[utoipa::path(
    get,
    path = "/api/v1/task/{uuid}",
    responses(
    (status = 200, response = TaskDetails)
    )
)]
pub async fn task_detail_handler(
    Path(uuid): Path<Uuid>,
    State(state): State<SharedAppState>,
) -> impl IntoResponse {
    let task_detail = state.task_manager.get_task_details(&uuid).await;
    if task_detail.is_none() {
        return Err(AppError::TaskNotFound(uuid.clone()));
    }
    let task_detail = task_detail.unwrap();
    Ok(Json(task_detail))
}
