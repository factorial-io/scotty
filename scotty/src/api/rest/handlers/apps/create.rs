use crate::{
    api::error::AppError,
    api::middleware::authorization::AuthorizationContext,
    api::secure_response::SecureJson,
    app_state::SharedAppState,
    docker::blueprint_template::TemplateInputs,
    docker::create_app::{create_app, CreateRequest},
    services::AuthorizationService,
};
use axum::{
    debug_handler,
    extract::{rejection::JsonRejection, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use flate2::read::GzDecoder;
use scotty_core::{
    apps::{
        app_data::AppSettings,
        create_app_request::CreateAppRequest,
        file_list::{File, FileList},
    },
    settings::loadbalancer::LoadBalancerType,
    tasks::running_app_context::RunningAppContext,
    utils::{format::format_bytes, slugify::slugify},
};
use std::io::Read;
use tracing::error;

/// Settings of an existing app with this name, read from its directory, or
/// `None` if there is none. A directory without `.scotty.yml` counts as an
/// app in scope `default` without a blueprint, as in `find_apps`.
fn existing_app_settings(root_folder: &str, app_name: &str) -> anyhow::Result<Option<AppSettings>> {
    let dir = std::path::Path::new(root_folder).join(slugify(app_name));
    if !dir.is_dir() {
        return Ok(None);
    }
    Ok(Some(
        AppSettings::from_file(&dir.join(".scotty.yml"))?.unwrap_or_default(),
    ))
}

/// Request fields a caller with only `create:<blueprint>` may not set, by
/// name. They would let the caller claim hostnames (custom domains, public
/// services), change what the blueprint's compose file interpolates or what
/// its containers see (environment; parameters are the interface for that),
/// use Scotty's registry credentials (registry), or make Scotty send requests
/// to arbitrary URLs (notify).
///
/// Allowed: TTL, basic auth, robots, allowlisted middlewares. The domain is
/// always derived from the app name.
fn blueprint_only_violations(payload: &CreateAppRequest) -> Vec<&'static str> {
    let settings = &payload.settings;
    [
        ("custom domains", !payload.custom_domains.is_empty()),
        ("public services", !settings.public_services.is_empty()),
        ("environment", !settings.environment.is_empty()),
        ("registry", settings.registry.is_some()),
        ("notifications", !settings.notify.is_empty()),
    ]
    .into_iter()
    .filter_map(|(field, set)| set.then_some(field))
    .collect()
}

/// Replaces the settings only Scotty manages with Scotty's values:
/// - `blueprint_hash` and `generated_parameters` are recorded for templated
///   blueprints by the create itself;
/// - custom actions are managed through their own endpoints and need
///   approval, so a caller can't bring its own (e.g. already approved). A
///   re-create keeps the existing app's.
fn drop_scotty_managed_settings(settings: &mut AppSettings, existing: Option<&AppSettings>) {
    settings.blueprint_hash = None;
    settings.generated_parameters.clear();
    settings.custom_actions = existing
        .map(|e| e.custom_actions.clone())
        .unwrap_or_default();
}

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
    let Json(mut payload) = payload.map_err(|rejection| {
        if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
            AppError::CreateAppPayloadTooLarge(
                declared_body_size(&headers),
                format_bytes(state.settings.api.create_app_max_size),
            )
        } else {
            AppError::BadRequest(rejection.body_text())
        }
    })?;

    // Check scope-based permissions before proceeding
    let user_id = AuthorizationService::get_user_id_for_authorization(&auth_context.user);
    let auth_service = &state.auth_service;

    // `create:<blueprint>` only covers templated blueprints without a caller
    // compose file; anything else needs plain `create` (D8).
    let templated_blueprint = payload
        .settings
        .app_blueprint
        .as_deref()
        .filter(|bp| {
            state
                .settings
                .apps
                .blueprints
                .get(*bp)
                .is_some_and(|b| b.template.is_some())
        })
        .filter(|_| {
            !payload
                .files
                .files
                .iter()
                .any(|f| crate::docker::blueprint_template::is_compose_file(&f.name))
        });
    let allowed = auth_service
        .check_create_in_scopes(&user_id, &payload.requested_scopes, templated_blueprint)
        .await;

    if !allowed {
        return Err(AppError::ScopeAccessDenied(format!(
            "User {} lacks create permission in scopes {:?}{}",
            auth_context.user.email,
            payload.requested_scopes,
            payload
                .settings
                .app_blueprint
                .as_deref()
                .map(|bp| format!(" for blueprint {bp}"))
                .unwrap_or_default()
        )));
    }

    // `create:<blueprint>` covers parameters, image and payload files; the
    // blueprint decides everything else that reaches the load balancer, the
    // containers or other systems.
    if templated_blueprint.is_some() {
        let fields = blueprint_only_violations(&payload);
        if !fields.is_empty()
            && !auth_service
                .check_create_in_scopes(&user_id, &payload.requested_scopes, None)
                .await
        {
            return Err(AppError::ScopeAccessDenied(format!(
                "setting {} needs the create permission",
                fields.join(", ")
            )));
        }
    }

    // One create per app at a time: the checks below decide on the app's
    // current state on disk, and the create task writes the new one.
    let lock = state
        .create_locks
        .try_lock(&slugify(&payload.app_name))
        .ok_or_else(|| AppError::CreateInProgress(payload.app_name.clone()))?;

    // Create is create-or-update. Updating an app needs rights on the app
    // being replaced, read from disk, not the in-memory list, which is empty
    // right after a restart (create-existing-app D1-D3).
    let existing = existing_app_settings(&state.settings.apps.root_folder, &payload.app_name)
        .map_err(|e| {
            // A broken file on disk is a server problem; keep its contents
            // out of the response.
            tracing::error!("Can't read .scotty.yml of app {}: {e}", payload.app_name);
            AppError::InternalServerError(format!(
                "Can't read the settings of existing app {}",
                payload.app_name
            ))
        })?;
    if let Some(existing) = &existing {
        let as_set = |s: &[String]| s.iter().cloned().collect::<std::collections::BTreeSet<_>>();
        if as_set(&existing.scopes) != as_set(&payload.requested_scopes) {
            return Err(AppError::ScopeAccessDenied(format!(
                "App {} exists in scopes {:?}; re-creating it can't change them to {:?}",
                payload.app_name, existing.scopes, payload.requested_scopes
            )));
        }
        // `create:<blueprint>` only covers apps created from that blueprint.
        let candidate =
            templated_blueprint.filter(|bp| existing.app_blueprint.as_deref() == Some(*bp));
        // Rights in every scope of the app being replaced, not just one.
        if !auth_service
            .check_create_in_every_scope(&user_id, &existing.scopes, candidate)
            .await
        {
            return Err(AppError::ScopeAccessDenied(format!(
                "User {} may not replace existing app {}{}",
                auth_context.user.email,
                payload.app_name,
                existing
                    .app_blueprint
                    .as_deref()
                    .map(|bp| format!(" (blueprint {bp})"))
                    .unwrap_or_default()
            )));
        }
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

    let max_decompressed_size = state.settings.api.create_app_max_size;

    let files: Result<Vec<File>, AppError> = payload
        .files
        .files
        .iter()
        .map(|f| {
            // Content arrives base64-encoded; legacy clients wrapped it twice
            // (see `FileContent`), which `decode` unwraps.
            let decoded = f.content.decode().map_err(|e| {
                error!("Failed to decode base64 content for {}: {}", f.name, e);
                AppError::FileContentDecodingError
            })?;

            // Then decompress if needed
            let content = if f.compressed {
                let decoder = GzDecoder::new(&decoded[..]);
                let mut decompressed = Vec::new();

                // Try to read max_size + 1 bytes to detect if file exceeds limit
                let mut limited_reader = decoder.take((max_decompressed_size + 1) as u64);
                let bytes_read = limited_reader.read_to_end(&mut decompressed).map_err(|e| {
                    error!("Failed to decompress content for {}: {}", f.name, e);
                    AppError::FileCompressionCorrupted(f.name.clone(), e.to_string())
                })?;

                // If we read more than the limit, the file is too large
                if bytes_read > max_decompressed_size {
                    error!(
                        "File {} exceeds maximum decompressed size of {} bytes (actual: {} bytes)",
                        f.name, max_decompressed_size, bytes_read
                    );
                    return Err(AppError::FileDecompressedSizeExceeded(
                        f.name.clone(),
                        max_decompressed_size,
                    ));
                }

                decompressed
            } else {
                decoded.into_owned()
            };

            Ok(File {
                name: f.name.clone(),
                content: content.into(),
                compressed: false, // After decompression, content is uncompressed
            })
        })
        .collect();

    let files = files?;

    let file_list = FileList { files };

    // Set the default settings for the app.
    let mut settings = payload.settings.clone();
    drop_scotty_managed_settings(&mut settings, existing.as_ref());
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

    let request = CreateRequest {
        app_name: &payload.app_name,
        settings: &settings,
        files: &file_list,
        template: TemplateInputs {
            parameters: Some(&payload.blueprint_parameters),
            image: payload.image.as_deref(),
            existing: existing.as_ref(),
        },
        requested_by: Some(user_id.clone()),
        lock: Some(lock),
    };
    match create_app(state, request).await {
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

    fn action(name: &str) -> scotty_core::settings::custom_action::CustomAction {
        let mut action = scotty_core::settings::custom_action::CustomAction::new(
            name.into(),
            String::new(),
            [("web".to_string(), vec!["true".to_string()])].into(),
            serde_json::from_value(serde_json::json!("action_write")).unwrap(),
            "someone".into(),
        );
        action.approve("someone".into(), None);
        action
    }

    #[test]
    fn caller_cannot_set_scotty_managed_settings() {
        let mut caller = AppSettings {
            blueprint_hash: Some("forged".into()),
            generated_parameters: vec!["X".into()],
            ..Default::default()
        };
        caller
            .custom_actions
            .insert("sneaky".into(), action("sneaky"));

        let mut fresh = caller.clone();
        drop_scotty_managed_settings(&mut fresh, None);
        assert!(fresh.blueprint_hash.is_none());
        assert!(fresh.generated_parameters.is_empty());
        assert!(fresh.custom_actions.is_empty());

        // A re-create keeps the existing app's actions, not the caller's.
        let mut existing = AppSettings::default();
        existing
            .custom_actions
            .insert("approved".into(), action("approved"));
        let mut re_create = caller.clone();
        drop_scotty_managed_settings(&mut re_create, Some(&existing));
        let names: Vec<_> = re_create.custom_actions.keys().collect();
        assert_eq!(names, ["approved"]);
    }

    #[test]
    fn blueprint_only_callers_may_set_only_harmless_fields() {
        let request = |settings: serde_json::Value, custom_domains: serde_json::Value| {
            let mut base = serde_json::json!({
                "public_services": [], "domain": "", "time_to_live": "Forever",
                "basic_auth": null, "disallow_robots": true, "environment": {},
                "registry": null, "app_blueprint": "bp",
            });
            base.as_object_mut()
                .unwrap()
                .extend(settings.as_object().unwrap().clone());
            serde_json::from_value::<CreateAppRequest>(serde_json::json!({
                "app_name": "a",
                "settings": base,
                "files": {"files": []},
                "custom_domains": custom_domains,
                "requested_scopes": ["default"],
            }))
            .unwrap()
        };
        let none = serde_json::json!([]);

        // TTL, basic auth, robots, middlewares and the domain are fine.
        let harmless = request(
            serde_json::json!({
                "time_to_live": {"Days": 1}, "basic_auth": ["u", "p"],
                "disallow_robots": false, "middlewares": ["x"], "domain": "evil.org",
            }),
            none.clone(),
        );
        assert!(blueprint_only_violations(&harmless).is_empty());

        for (settings, domains, field) in [
            (
                serde_json::json!({}),
                serde_json::json!([{"domain": "login.example.com", "service": "web"}]),
                "custom domains",
            ),
            (
                serde_json::json!({"public_services": [{"service": "web", "port": 80, "domains": []}]}),
                none.clone(),
                "public services",
            ),
            (
                serde_json::json!({"environment": {"PORT": "1"}}),
                none.clone(),
                "environment",
            ),
            (
                serde_json::json!({"registry": "private"}),
                none.clone(),
                "registry",
            ),
            (
                serde_json::json!({"notify": ["Log"]}),
                none.clone(),
                "notifications",
            ),
        ] {
            assert_eq!(
                blueprint_only_violations(&request(settings, domains)),
                [field]
            );
        }
    }
    use base64::prelude::*;
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

        // Create a File with compressed flag
        let file = File {
            name: "test.txt".to_string(),
            content: compressed.into(),
            compressed: true,
        };

        // Simulate the decompression logic from the handler
        let decoded = file.content.decode().unwrap();
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

        // Create a File without compression flag
        let file = File {
            name: "test.txt".to_string(),
            content: original_content.to_vec().into(),
            compressed: false,
        };

        // Simulate the passthrough logic from the handler
        let decoded = file.content.decode().unwrap();

        // Verify decoded content matches original
        assert_eq!(&*decoded, original_content);
    }

    /// A scottyctl <= 0.3.3 payload (base64 ASCII as a JSON int array) must
    /// still decode to the same bytes as the new string form.
    #[test]
    fn test_legacy_client_payload_still_decodes() {
        let original_content = b"Hello from an old client.";

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original_content).unwrap();
        let compressed = encoder.finish().unwrap();

        let legacy = serde_json::json!({
            "name": "test.txt",
            "content": BASE64_STANDARD.encode(&compressed).into_bytes(),
            "compressed": true,
        });
        let file: File = serde_json::from_value(legacy).unwrap();

        let decoded = file.content.decode().unwrap();
        let mut decompressed = Vec::new();
        GzDecoder::new(&decoded[..])
            .read_to_end(&mut decompressed)
            .unwrap();

        assert_eq!(decompressed, original_content);
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

    #[test]
    fn test_decompression_size_limit_check() {
        // Create large content that will definitely exceed a small limit
        let original_content = "This is a longer text that won't compress to nothing. ".repeat(100); // ~5.4KB
        let original_bytes = original_content.as_bytes();

        // Compress it
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original_bytes).unwrap();
        let compressed = encoder.finish().unwrap();

        // Use a very small limit to ensure we exceed it
        let max_decompressed_size = 100; // Only 100 bytes
        let decoder = GzDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();

        // Try to read max_size + 1 bytes
        let mut limited_reader = decoder.take((max_decompressed_size + 1) as u64);
        let bytes_read = limited_reader.read_to_end(&mut decompressed).unwrap();

        // Should have read more than the limit (file was too large)
        assert!(
            bytes_read > max_decompressed_size,
            "Should have read more than the limit ({} bytes), got {} bytes",
            max_decompressed_size,
            bytes_read
        );
        assert_eq!(
            bytes_read,
            max_decompressed_size + 1,
            "Should have read exactly limit + 1 bytes"
        );
    }

    #[test]
    fn test_decompression_within_size_limit() {
        // Create small content that won't exceed limit
        let original_content = b"Hello, World!";

        // Compress it
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original_content).unwrap();
        let compressed = encoder.finish().unwrap();

        // Simulate checking with generous limit
        let max_decompressed_size = 1024; // 1KB limit
        let decoder = GzDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();

        // Try to read max_size + 1 bytes
        let mut limited_reader = decoder.take((max_decompressed_size + 1) as u64);
        let bytes_read = limited_reader.read_to_end(&mut decompressed).unwrap();

        // Should have read less than or equal to the limit (file was within limit)
        assert!(
            bytes_read <= max_decompressed_size,
            "Should have read {} bytes or less, got {} bytes",
            max_decompressed_size,
            bytes_read
        );
        assert_eq!(
            decompressed, original_content,
            "Decompressed content should match original"
        );
    }
}
