use crate::{
    api::error::AppError,
    api::middleware::authorization::AuthorizationContext,
    api::secure_response::SecureJson,
    app_state::SharedAppState,
    docker::create_app::create_app,
    services::{authorization::Permission, AuthorizationService},
};
use axum::{debug_handler, extract::State, response::IntoResponse, Extension, Json};
use base64::prelude::*;
use scotty_core::{
    apps::{
        create_app_request::CreateAppRequest,
        file_list::{File, FileList},
    },
    settings::loadbalancer::LoadBalancerType,
    tasks::running_app_context::RunningAppContext,
};
use tracing::error;

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
    Json(mut payload): Json<CreateAppRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Check scope-based permissions before proceeding
    let user_id = AuthorizationService::get_user_id_for_authorization(&auth_context.user);
    let auth_service = &state.auth_service;

    let allowed = auth_service
        .check_permission_in_scopes(&user_id, &payload.requested_scopes, &Permission::Create)
        .await;

    if !allowed {
        return Err(AppError::ScopeAccessDenied(format!(
            "User {} lacks create permission in scopes: {:?}",
            auth_context.user.email, payload.requested_scopes
        )));
    }

    // Copy validated scopes to settings
    payload.settings.scopes = payload.requested_scopes.clone();

    // Check if any file is named .scotty.yml
    if payload
        .files
        .files
        .iter()
        .any(|f| f.name.ends_with(".scotty.yml"))
    {
        return Err(AppError::CantCreateAppWithScottyYmlFile);
    }

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

    // Set the default settings for the app.
    let settings = payload.settings.clone();
    let settings = settings.merge_with_global_settings(&state.settings.apps, &payload.app_name);

    // Apply blueprint settings, if any.
    let settings = settings.apply_blueprint(&state.settings.apps.blueprints)?;

    // Apply custom domains, if any.
    let settings = settings.apply_custom_domains(&payload.custom_domains)?;

    if state.settings.load_balancer_type == LoadBalancerType::Traefik
        && !settings.middlewares.is_empty()
    {
        // Check if the middlewares are listed in settings.traefik.allowed_middlewares
        for middleware in &settings.middlewares {
            if !state
                .settings
                .traefik
                .allowed_middlewares
                .contains(middleware)
            {
                return Err(AppError::MiddlewareNotAllowed(middleware.clone()));
            }
        }
    }

    match create_app(state, &payload.app_name, &settings, &file_list).await {
        Ok(app_data) => Ok(SecureJson(app_data)),
        Err(e) => {
            error!("App create failed with: {:?}", e);
            Err(AppError::from(e))
        }
    }
}
