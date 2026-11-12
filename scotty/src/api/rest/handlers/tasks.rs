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
    services::{apps, AuthorizationService},
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
    apps::task(&state, &user_id, &uuid).await.map(Json)
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
    Ok(Json(TaskList {
        tasks: apps::tasks(&state, &user_id).await,
    }))
}
