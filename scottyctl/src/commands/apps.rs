use anyhow::Context;
use owo_colors::OwoColorize;
use tabled::{builder::Builder, settings::Style};

use crate::{
    api::{get, get_or_post, wait_for_task},
    cli::{
        ActionCommand, AdoptCommand, CreateCommand, DestroyCommand, InfoCommand, PurgeCommand,
        RebuildCommand, RunCommand, StopCommand,
    },
    context::{AppContext, ServerSettings},
    utils::{
        files::collect_files,
        formatting::{colored_by_status, format_since},
        parsers::parse_env_file,
    },
};
use base64::prelude::*;
use scotty_core::{
    apps::{
        app_data::{AppData, AppSettings},
        create_app_request::CreateAppRequest,
        file_list::{File, FileList},
        shared_app_list::AppDataVec,
    },
    tasks::running_app_context::RunningAppContext,
};

pub async fn list_apps(context: &AppContext) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Getting list of apps from {} ...",
        context.server().server
    ));
    ui.run(async || {
        let result = get(context.server(), "apps/list").await?;

        let apps: AppDataVec =
            serde_json::from_value(result).context("Failed to parse apps list")?;

        let mut builder = Builder::default();
        builder.push_record(vec!["Name", "Status", "Since", "URLs"]);
        for app in apps.apps {
            let urls = app.urls();
            builder.push_record(vec![
                &colored_by_status(&app.name, &app.status),
                &app.status.to_string(),
                &format_since(&app.running_since()),
                &urls.join("\n"),
            ]);
        }

        let mut table = builder.build();

        table.with(Style::rounded());

        ui.success("Got all apps!");
        Ok(table.to_string())
    })
    .await
}

pub async fn call_apps_api(context: &AppContext, verb: &str, app_name: &str) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Running action {} for app {} at {} ...",
        verb.yellow(),
        app_name.yellow(),
        context.server().server.yellow()
    ));
    ui.run(async || {
        let result = get(context.server(), &format!("apps/{}/{}", verb, app_name)).await?;
        let app_context: RunningAppContext =
            serde_json::from_value(result).context("Failed to parse context from API")?;
        wait_for_task(context.server(), &app_context, ui).await?;
        let app_data = get_app_info(context.server(), &app_context.app_data.name).await?;
        ui.success(format!(
            "{} action for app {} has been successfully completed!",
            verb.yellow(),
            app_name.yellow()
        ));
        format_app_info(&app_data)
    })
    .await
}

pub async fn get_app_info(server: &ServerSettings, app_name: &str) -> anyhow::Result<AppData> {
    let app_data = get(server, &format!("apps/info/{}", app_name)).await?;
    let app_data: AppData = serde_json::from_value(app_data).context("Failed to parse app data")?;

    Ok(app_data)
}

pub fn format_app_info(app_data: &AppData) -> anyhow::Result<String> {
    let mut builder = Builder::default();
    builder.push_record(vec!["Service", "Status", "Running since", "URL"]);
    for service in &app_data.services {
        let urls = service.get_urls();
        builder.push_record(vec![
            &service.service,
            &service.status.to_string(),
            &format_since(&service.running_since()),
            &urls.join("\n"),
        ]);
    }

    let mut table = builder.build();
    table.with(Style::rounded());

    let mut result = format!("Info for {}\n{}", app_data.name, table);

    if app_data.settings.is_some() && !app_data.settings.as_ref().unwrap().notify.is_empty() {
        result += "\nNotification services";
        let mut builder = Builder::default();
        builder.push_record(["Type", "Service-Id", "Context"]);
        for notification in &app_data.settings.as_ref().unwrap().notify {
            #[allow(unused_assignments)]
            let mut context: String = "".into();
            builder.push_record(match notification {
                scotty_core::notification_types::NotificationReceiver::Log => ["Log", "Log", ""],
                scotty_core::notification_types::NotificationReceiver::Webhook(ctx) => {
                    ["Webhook", &ctx.service_id, ""]
                }
                scotty_core::notification_types::NotificationReceiver::Mattermost(ctx) => {
                    ["Mattermost", &ctx.service_id, &ctx.channel]
                }
                scotty_core::notification_types::NotificationReceiver::Gitlab(ctx) => {
                    context = format!("Project-Id: {}  MR-Id: {}", ctx.project_id, ctx.mr_id);
                    ["Gitlab", &ctx.service_id, &context]
                }
            });
        }
        let table = builder.build().with(Style::rounded()).to_string();
        result += format!("\n{}", table).as_str();
    }

    Ok(result)
}

pub async fn rebuild_app(context: &AppContext, cmd: &RebuildCommand) -> anyhow::Result<()> {
    call_apps_api(context, "rebuild", &cmd.app_name).await
}

pub async fn run_app(context: &AppContext, cmd: &RunCommand) -> anyhow::Result<()> {
    call_apps_api(context, "run", &cmd.app_name).await
}

pub async fn stop_app(context: &AppContext, cmd: &StopCommand) -> anyhow::Result<()> {
    call_apps_api(context, "stop", &cmd.app_name).await
}

pub async fn purge_app(context: &AppContext, cmd: &PurgeCommand) -> anyhow::Result<()> {
    call_apps_api(context, "purge", &cmd.app_name).await
}

pub async fn adopt_app(context: &AppContext, cmd: &AdoptCommand) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!("Adopting app {}...", &cmd.app_name));
    ui.run(async || {
        let result = get(context.server(), &format!("apps/adopt/{}", &cmd.app_name)).await?;
        let app_data: AppData = serde_json::from_value(result)?;
        ui.success(format!("App {} adopted successfully", &cmd.app_name));
        format_app_info(&app_data)
    })
    .await
}

pub async fn info_app(context: &AppContext, cmd: &InfoCommand) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Getting info for app {}...",
        &cmd.app_name.yellow()
    ));
    ui.run(async || {
        let result = get(context.server(), &format!("apps/info/{}", cmd.app_name)).await?;
        let app_data: AppData = serde_json::from_value(result)?;
        ui.success(format!(
            "Info for app {} received successfully",
            &cmd.app_name.yellow()
        ));
        format_app_info(&app_data)
    })
    .await
}

pub async fn destroy_app(context: &AppContext, cmd: &DestroyCommand) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!("Destroying app {}...", &cmd.app_name.yellow()));
    ui.run(async || {
        let result = get(context.server(), &format!("apps/destroy/{}", &cmd.app_name)).await?;
        let app_context: RunningAppContext =
            serde_json::from_value(result).context("Failed to parse context from API")?;
        wait_for_task(context.server(), &app_context, ui).await?;
        ui.success(format!(
            "App {} destroyed successfully",
            &cmd.app_name.yellow()
        ));

        Ok(format!("App {} destroyed", &cmd.app_name))
    })
    .await
}

pub async fn run_custom_action(context: &AppContext, cmd: &ActionCommand) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Running custom action {} on app {}...",
        &cmd.action_name.yellow(),
        &cmd.app_name.yellow()
    ));
    ui.run(async || {
        let payload = serde_json::json!({
            "action_name": cmd.action_name
        });

        let result = get_or_post(
            context.server(),
            &format!("apps/{}/actions", &cmd.app_name),
            "POST",
            Some(payload),
        )
        .await?;

        let app_context: RunningAppContext =
            serde_json::from_value(result).context("Failed to parse context from API")?;

        wait_for_task(context.server(), &app_context, ui).await?;

        let app_data = get_app_info(context.server(), &app_context.app_data.name).await?;

        ui.success(format!(
            "Custom action {} completed successfully for app {}",
            &cmd.action_name.yellow(),
            &cmd.app_name.yellow()
        ));

        format_app_info(&app_data)
    })
    .await
}

pub async fn create_app(context: &AppContext, cmd: &CreateCommand) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!("Creating app {}...", &cmd.app_name.yellow()));
    ui.run(async || {
        ui.new_status_line("Collecting files...");
        let file_list = collect_files(&cmd.docker_compose_path)?;
        // Encode content base64
        let file_list = FileList {
            files: file_list
                .files
                .iter()
                .map(|f| File {
                    name: f.name.clone(),
                    content: BASE64_STANDARD.encode(&f.content).into(),
                })
                .collect(),
        };
        ui.success(format!("{} files ready to beam.", file_list.files.len()));

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

        let payload = CreateAppRequest {
            app_name: cmd.app_name.clone(),
            custom_domains: cmd.custom_domain.clone(),
            settings: AppSettings {
                public_services: cmd.service.clone(),
                basic_auth: cmd.basic_auth.clone(),
                environment: environment.iter().cloned().collect(),
                registry: cmd.registry.clone(),
                app_blueprint: cmd.app_blueprint.clone(),
                time_to_live: cmd.ttl.clone(),
                disallow_robots: !cmd.allow_robots,
                destroy_on_ttl: cmd.destroy_on_ttl,
                ..Default::default()
            },
            files: file_list,
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
