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

    // Set the domain.
    let settings = payload.settings.clone();
    let settings = settings.merge_with_global_settings(&state.settings.apps, &payload.app_name);

    match create_app(state, &payload.app_name, &settings, &file_list).await {
        Ok(app_data) => Ok(Json(app_data)),
        Err(e) => {
            error!("App create failed with: {:?}", e);
            Err(AppError::from(e))
        }
    }
}
