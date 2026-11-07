use axum::{
    debug_handler,
    extract::{Path, State},
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::{error::AppError, middleware::authorization::AuthorizationContext},
    app_state::SharedAppState,
    services::AuthorizationService,
};
use scotty_core::tasks::task_details::TaskDetails;

#[utoipa::path(
    get,
    path = "/api/v1/authenticated/task/{uuid}",
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
    Extension(auth_context): Extension<AuthorizationContext>,
) -> impl IntoResponse {
    let user_id = AuthorizationService::get_user_id_for_authorization(&auth_context.user);
    // A task the caller may not see answers like a missing one, so its
    // existence is not revealed.
    match state.task_manager.get_task_details(&uuid).await {
        Some(task) if state.auth_service.can_view_task(&user_id, &task).await => Ok(Json(task)),
        _ => Err(AppError::TaskNotFound(uuid)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToResponse, utoipa::ToSchema)]
pub struct TaskList {
    pub tasks: Vec<TaskDetails>,
}

#[utoipa::path(
    get,
    path = "/api/v1/authenticated/tasks",
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
    Extension(auth_context): Extension<AuthorizationContext>,
) -> Result<Json<TaskList>, AppError> {
    let user_id = AuthorizationService::get_user_id_for_authorization(&auth_context.user);
    let mut tasks = Vec::new();
    for task in state.task_manager.get_task_list().await {
        if state.auth_service.can_view_task(&user_id, &task).await {
            tasks.push(task);
        }
    }
    Ok(Json(TaskList { tasks }))
}
