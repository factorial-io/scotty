use anyhow::Context;
use owo_colors::OwoColorize;

use crate::{
    api::{get, get_or_post, wait_for_task},
    cli::CreateCommand,
    context::AppContext,
    utils::{files::collect_files, parsers::parse_env_file},
};
use flate2::write::GzEncoder;
use flate2::Compression;
use scotty_core::{
    apps::{
        app_data::AppSettings,
        create_app_request::CreateAppRequest,
        file_list::{File, FileList},
    },
    settings::app_blueprint::AppBlueprintList,
    tasks::running_app_context::RunningAppContext,
    utils::secret::SecretHashMap,
};
use std::io::Write;

use super::{format_app_info, get_app_info};

/// Turns `--param KEY=VALUE` strings into JSON values typed by the
/// blueprint's schema (`integer`, `number`, `boolean`; anything else stays a
/// string). The server validates the result against the same schema.
fn typed_parameters(
    params: &[(String, String)],
    schema: Option<&serde_json::Value>,
) -> anyhow::Result<serde_json::Map<String, serde_json::Value>> {
    use serde_json::Value;
    params
        .iter()
        .map(|(key, raw)| {
            let ty = schema
                .and_then(|s| s.pointer(&format!("/properties/{key}/type")))
                .and_then(Value::as_str);
            let value = match ty {
                Some("integer" | "number" | "boolean") => serde_json::from_str(raw)
                    .with_context(|| format!("--param {key}: {raw:?} is not a {}", ty.unwrap()))?,
                _ => Value::String(raw.clone()),
            };
            Ok((key.clone(), value))
        })
        .collect()
}

/// Create a new app
pub async fn create_app(context: &AppContext, cmd: &CreateCommand) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!("Creating app {}...", cmd.app_name.yellow()));
    ui.run(async || {
        ui.new_status_line("Collecting files...");
        // ponytail: a templated blueprint may take no files at all; whether a
        // compose file is required or forbidden is the server's call.
        let file_list = match &cmd.folder {
            Some(folder) => collect_files(std::path::Path::new(folder))?,
            None => FileList { files: vec![] },
        };
        let blueprint_parameters = match &cmd.app_blueprint {
            Some(bp) if !cmd.param.is_empty() => {
                let result = get(context.server(), "blueprints").await?;
                let blueprints: AppBlueprintList = serde_json::from_value(result)?;
                let schema = blueprints
                    .blueprints
                    .get(bp)
                    .and_then(|b| b.template.as_ref())
                    .and_then(|t| t.parameters.as_ref());
                typed_parameters(&cmd.param, schema)?
            }
            _ if !cmd.param.is_empty() => {
                anyhow::bail!("--param needs --app-blueprint naming a templated blueprint")
            }
            _ => Default::default(),
        };

        // Compress and encode files
        let mut total_original_size = 0;
        let mut total_compressed_size = 0;

        let file_list = FileList {
            files: file_list
                .files
                .iter()
                .map(|f| -> anyhow::Result<File> {
                    total_original_size += f.content.len();

                    // Compress with gzip
                    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                    encoder
                        .write_all(&f.content)
                        .context(format!("Failed to compress file: {}", f.name))?;
                    let compressed = encoder
                        .finish()
                        .context(format!("Failed to finish compression for: {}", f.name))?;

                    total_compressed_size += compressed.len();

                    Ok(File {
                        name: f.name.clone(),
                        // Raw gzip bytes; serialization base64-encodes them.
                        content: compressed.into(),
                        compressed: true,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
        };

        let compression_ratio = if total_original_size > 0 {
            (100.0 - (total_compressed_size as f64 / total_original_size as f64 * 100.0)) as u32
        } else {
            0
        };

        ui.success(format!(
            "{} files ready to beam (compressed {}%, {} → {}).",
            file_list.files.len(),
            compression_ratio,
            scotty_core::utils::format::format_bytes(total_original_size),
            scotty_core::utils::format::format_bytes(total_compressed_size)
        ));

        // Combine environment variables from env-file and command line
        let mut environment = cmd.env.clone();

        // Add environment variables from env-file if specified
        if let Some(env_file_path) = &cmd.env_file {
            ui.new_status_line("Collecting env-file...");
            match parse_env_file(env_file_path) {
                Ok(env_file_vars) => {
                    ui.success(format!(
                        "Loaded {} environment variables from {}",
                        env_file_vars.len().to_string().green(),
                        env_file_path.yellow()
                    ));
                    let mut combined_env = env_file_vars;
                    combined_env.extend(environment.iter().cloned());
                    environment = combined_env;
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Failed to parse env file {}: {}",
                        env_file_path,
                        e
                    ));
                }
            }
        }

        // Use default scope if none specified
        let requested_scopes = if cmd.scope.is_empty() {
            vec!["default".to_string()]
        } else {
            cmd.scope.clone()
        };

        let payload = CreateAppRequest {
            app_name: cmd.app_name.clone(),
            custom_domains: cmd.custom_domain.clone(),
            settings: AppSettings {
                public_services: cmd.service.clone(),
                basic_auth: cmd.basic_auth.clone(),
                environment: SecretHashMap::from_hashmap(environment.iter().cloned().collect()),
                registry: cmd.registry.clone(),
                app_blueprint: cmd.app_blueprint.clone(),
                time_to_live: cmd.ttl.clone(),
                disallow_robots: !cmd.allow_robots,
                destroy_on_ttl: cmd.destroy_on_ttl,
                middlewares: cmd.middleware.clone(),
                ..Default::default()
            },
            files: file_list,
            requested_scopes,
            blueprint_parameters,
            image: cmd.image.clone(),
        };

        let payload = serde_json::to_value(&payload).context("Failed to serialize payload")?;
        let size = scotty_core::utils::format::format_bytes(payload.to_string().len());

        // Connect WebSocket before starting the task
        let ws_connection =
            crate::websocket::AuthenticatedWebSocket::connect(context.server()).await;

        ui.new_status_line(format!(
            "Beaming your app {} up to {} ({})...",
            cmd.app_name.yellow(),
            context.server().server.yellow(),
            size.blue()
        ));
        let result = get_or_post(context.server(), "apps/create", "POST", Some(payload)).await?;

        ui.success(format!(
            "App {} beamed up to {} ({})!",
            cmd.app_name.yellow(),
            context.server().server.yellow(),
            size.blue()
        ));
        ui.new_status_line(format!(
            "Waiting for app {} to start...",
            cmd.app_name.yellow()
        ));
        let app_context: RunningAppContext =
            serde_json::from_value(result).context("Failed to parse context from API")?;

        wait_for_task(context.server(), &app_context, ui, ws_connection).await?;
        let app_data = get_app_info(context.server(), &app_context.app_data.name).await?;
        ui.success(format!(
            "App {} started successfully!",
            cmd.app_name.yellow(),
        ));

        format_app_info(&app_data)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::typed_parameters;
    use serde_json::json;

    #[test]
    fn params_are_typed_by_schema() {
        let schema = json!({"properties": {
            "N": {"type": "integer"}, "B": {"type": "boolean"}, "S": {"type": "string"}
        }});
        let p = |k: &str, v: &str| (k.to_string(), v.to_string());
        let out = typed_parameters(
            &[p("N", "3"), p("B", "true"), p("S", "42"), p("X", "x")],
            Some(&schema),
        )
        .unwrap();
        assert_eq!(out["N"], json!(3));
        assert_eq!(out["B"], json!(true));
        assert_eq!(out["S"], json!("42"));
        assert_eq!(out["X"], json!("x"));
        assert!(typed_parameters(&[p("N", "abc")], Some(&schema)).is_err());
    }
}
