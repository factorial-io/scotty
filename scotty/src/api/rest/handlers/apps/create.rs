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
use flate2::read::GzDecoder;
use scotty_core::{
    apps::{
        create_app_request::CreateAppRequest,
        file_list::{File, FileList},
    },
    settings::loadbalancer::LoadBalancerType,
    tasks::running_app_context::RunningAppContext,
};
use std::io::Read;
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
        .filter_map(|f| {
            // First decode base64
            let decoded = match BASE64_STANDARD.decode(&f.content) {
                Ok(d) => d,
                Err(e) => {
                    error!("Failed to decode base64 content for {}: {}", f.name, e);
                    return None;
                }
            };

            // Then decompress if needed
            let content = if f.compressed {
                let mut decoder = GzDecoder::new(&decoded[..]);
                let mut decompressed = Vec::new();
                match decoder.read_to_end(&mut decompressed) {
                    Ok(_) => decompressed,
                    Err(e) => {
                        error!("Failed to decompress content for {}: {}", f.name, e);
                        return None;
                    }
                }
            } else {
                decoded
            };

            Some(File {
                name: f.name.clone(),
                content,
                compressed: false, // After decompression, content is uncompressed
            })
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

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    #[test]
    fn test_decompress_compressed_file() {
        // Original content
        let original_content = b"Hello, World! This is a test file with some content.";

        // Compress it
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original_content).unwrap();
        let compressed = encoder.finish().unwrap();

        // Encode as base64
        let base64_encoded = BASE64_STANDARD.encode(&compressed);

        // Create a File with compressed flag
        let file = File {
            name: "test.txt".to_string(),
            content: base64_encoded.into_bytes(),
            compressed: true,
        };

        // Simulate the decompression logic from the handler
        let decoded = BASE64_STANDARD.decode(&file.content).unwrap();
        let mut decoder = GzDecoder::new(&decoded[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();

        // Verify decompressed content matches original
        assert_eq!(decompressed, original_content);
    }

    #[test]
    fn test_uncompressed_file_passthrough() {
        // Original content
        let original_content = b"Hello, World! This is not compressed.";

        // Encode as base64 directly
        let base64_encoded = BASE64_STANDARD.encode(original_content);

        // Create a File without compression flag
        let file = File {
            name: "test.txt".to_string(),
            content: base64_encoded.into_bytes(),
            compressed: false,
        };

        // Simulate the passthrough logic from the handler
        let decoded = BASE64_STANDARD.decode(&file.content).unwrap();

        // Verify decoded content matches original
        assert_eq!(decoded, original_content);
    }

    #[test]
    fn test_compression_saves_space() {
        // Create content with repetition (compresses well)
        let original_content = "Hello World! ".repeat(100);
        let original_bytes = original_content.as_bytes();

        // Compress it
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original_bytes).unwrap();
        let compressed = encoder.finish().unwrap();

        // Verify compression actually reduces size
        assert!(
            compressed.len() < original_bytes.len(),
            "Compressed size ({}) should be less than original size ({})",
            compressed.len(),
            original_bytes.len()
        );

        // Verify we can decompress back to original
        let mut decoder = GzDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();
        assert_eq!(decompressed, original_bytes);
    }
}
