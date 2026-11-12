use crate::{
    api::error::AppError,
    api::middleware::authorization::AuthorizationContext,
    api::secure_response::SecureJson,
    app_state::SharedAppState,
    services::{apps, AuthorizationService},
};
use axum::{
    debug_handler,
    extract::{rejection::JsonRejection, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use scotty_core::{
    apps::create_app_request::CreateAppRequest, tasks::running_app_context::RunningAppContext,
    utils::format::format_bytes,
};

/// Human-readable `Content-Length`, for error messages only.
fn declared_body_size(headers: &HeaderMap) -> String {
    headers
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok())
        .map(format_bytes)
        .unwrap_or_else(|| "unknown size".to_string())
}

#[utoipa::path(
    post,
    path = "/api/v1/authenticated/apps/create",
    request_body(content = CreateAppRequest, content_type = "application/json"),
    responses(
    (status = 200, response = inline(RunningAppContext)),
    (status = 401, description = "Access token is missing or invalid"),
    ),
    security(
            ("bearerAuth" = [])
        )
)]
#[debug_handler]
pub async fn create_app_handler(
    State(state): State<SharedAppState>,
    Extension(auth_context): Extension<AuthorizationContext>,
    headers: HeaderMap,
    payload: Result<Json<CreateAppRequest>, JsonRejection>,
) -> Result<impl IntoResponse, AppError> {
    // Axum's own 413 says only "length limit exceeded", which leaves no clue
    // about how big the body was or which setting caps it.
    let Json(payload) = payload.map_err(|rejection| {
        if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
            AppError::CreateAppPayloadTooLarge(
                declared_body_size(&headers),
                format_bytes(state.settings.api.create_app_max_size),
            )
        } else {
            AppError::BadRequest(rejection.body_text())
        }
    })?;

    let user_id = AuthorizationService::get_user_id_for_authorization(&auth_context.user);
    Ok(SecureJson(
        apps::create(&state, &user_id, payload, apps::OnExisting::Update).await?,
    ))
}
