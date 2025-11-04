use anyhow::Context;
use owo_colors::OwoColorize;

use crate::{
    api::{get_or_post, wait_for_task},
    cli::CreateCommand,
    context::AppContext,
    utils::{files::collect_files, parsers::parse_env_file},
};
use base64::prelude::*;
use flate2::write::GzEncoder;
use flate2::Compression;
use scotty_core::{
    apps::{
        app_data::AppSettings,
        create_app_request::CreateAppRequest,
        file_list::{File, FileList},
    },
    tasks::running_app_context::RunningAppContext,
    utils::secret::SecretHashMap,
};
use std::io::Write;

use super::{format_app_info, get_app_info};

/// Create a new app
pub async fn create_app(context: &AppContext, cmd: &CreateCommand) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!("Creating app {}...", &cmd.app_name.yellow()));
    ui.run(async || {
        ui.new_status_line("Collecting files...");
        let file_list = collect_files(&cmd.docker_compose_path)?;

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
                        content: BASE64_STANDARD.encode(&compressed).into(),
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
        };

        let payload = serde_json::to_value(&payload).context("Failed to serialize payload")?;
        let size = scotty_core::utils::format::format_bytes(payload.to_string().len());
        ui.new_status_line(format!(
            "Beaming your app {} up to {} ({})...",
            &cmd.app_name.yellow(),
            &context.server().server.yellow(),
            size.blue()
        ));
        let result = get_or_post(context.server(), "apps/create", "POST", Some(payload)).await?;

        ui.success(format!(
            "App {} beamed up to {} ({})!",
            &cmd.app_name.yellow(),
            &context.server().server.yellow(),
            size.blue()
        ));
        ui.new_status_line(format!(
            "Waiting for app {} to start...",
            &cmd.app_name.yellow()
        ));
        let app_context: RunningAppContext =
            serde_json::from_value(result).context("Failed to parse context from API")?;

        wait_for_task(context.server(), &app_context, ui).await?;
        let app_data = get_app_info(context.server(), &app_context.app_data.name).await?;
        ui.success(format!(
            "App {} started successfully!",
            &cmd.app_name.yellow(),
        ));

        format_app_info(&app_data)
    })
    .await
}
