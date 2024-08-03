use crate::{
    api::error::AppError,
    app_state::SharedAppState,
    apps::{
        create_app_request::CreateAppRequest,
        file_list::{File, FileList},
    },
    docker::create_app::create_app,
    tasks::running_app_context::RunningAppContext,
};
use axum::{debug_handler, extract::State, response::IntoResponse, Json};
use base64::prelude::*;
use tracing::error;

#[utoipa::path(
    post,
    path = "/api/v1/apps/create",
    request_body(content = CreateAppRequest, content_type = "application/json"),
    responses(
    (status = 200, response = inline(RunningAppContext))
    )
)]
#[debug_handler]
pub async fn create_app_handler(
    State(state): State<SharedAppState>,
    Json(payload): Json<CreateAppRequest>,
) -> Result<impl IntoResponse, AppError> {
    let files = payload
        .files
        .files
        .iter()
        .filter_map(|f| match BASE64_STANDARD.decode(&f.content) {
            Ok(decoded) => Some(File {
                name: f.name.clone(),
                content: decoded,
            }),
            Err(e) => {
                error!("Failed to decode base64 content: {}", e);
                None
            }
        })
        .collect::<Vec<_>>();

    let file_list = FileList { files };
    let app_data = create_app(state, &payload.app_name, &payload.settings, &file_list).await?;
    Ok(Json(app_data))
}
