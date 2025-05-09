use anyhow::Context;
use owo_colors::{OwoColorize, Style as OwoStyle};
use tabled::{builder::Builder, settings::Style};

use crate::{
    api::{get, get_or_post, wait_for_task},
    cli::{
        AdoptCommand, CreateCommand, DestroyCommand, InfoCommand, PurgeCommand, RebuildCommand,
        RunCommand, StopCommand,
    },
    progress_println,
    utils::{
        files::collect_files,
        formatting::{colored_by_status, format_since},
        parsers::parse_env_file,
        progress_tracker::ProgressTracker,
    },
    ServerSettings,
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

pub async fn list_apps(server: &ServerSettings) -> anyhow::Result<()> {
    let result = get(server, "apps/list").await?;

    let apps: AppDataVec = serde_json::from_value(result).context("Failed to parse apps list")?;

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

    println!("{}", table);

    Ok(())
}

pub async fn call_apps_api(
    server: &ServerSettings,
    verb: &str,
    app_name: &str,
) -> anyhow::Result<()> {
    let result = get(server, &format!("apps/{}/{}", verb, app_name)).await?;
    let context: RunningAppContext =
        serde_json::from_value(result).context("Failed to parse context from API")?;
    wait_for_task(server, &context).await?;
    let app_data = get_app_info(server, &context.app_data.name).await?;
    print_app_info(&app_data)?;
    Ok(())
}

pub async fn get_app_info(server: &ServerSettings, app_name: &str) -> anyhow::Result<AppData> {
    let app_data = get(server, &format!("apps/info/{}", app_name)).await?;
    let app_data: AppData = serde_json::from_value(app_data).context("Failed to parse app data")?;

    Ok(app_data)
}

pub fn print_app_info(app_data: &AppData) -> anyhow::Result<()> {
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

    println!("Info for {}", app_data.name);
    println!("{}", table);

    if app_data.settings.is_some() && !app_data.settings.as_ref().unwrap().notify.is_empty() {
        println!("Notification services");
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
        println!("{}", builder.build().with(Style::rounded()));
    }

    Ok(())
}

pub async fn rebuild_app(server: &ServerSettings, cmd: &RebuildCommand) -> anyhow::Result<()> {
    call_apps_api(server, "rebuild", &cmd.app_name).await
}

pub async fn run_app(server: &ServerSettings, cmd: &RunCommand) -> anyhow::Result<()> {
    call_apps_api(server, "run", &cmd.app_name).await
}

pub async fn stop_app(server: &ServerSettings, cmd: &StopCommand) -> anyhow::Result<()> {
    call_apps_api(server, "stop", &cmd.app_name).await
}

pub async fn purge_app(server: &ServerSettings, cmd: &PurgeCommand) -> anyhow::Result<()> {
    call_apps_api(server, "purge", &cmd.app_name).await
}

pub async fn adopt_app(server: &ServerSettings, cmd: &AdoptCommand) -> anyhow::Result<()> {
    let result = get(server, &format!("apps/adopt/{}", &cmd.app_name)).await?;
    let app_data: AppData = serde_json::from_value(result)?;
    print_app_info(&app_data)?;
    Ok(())
}

pub async fn info_app(server: &ServerSettings, cmd: &InfoCommand) -> anyhow::Result<()> {
    let result = get(server, &format!("apps/info/{}", cmd.app_name)).await?;
    let app_data: AppData = serde_json::from_value(result)?;
    print_app_info(&app_data)?;
    Ok(())
}

pub async fn destroy_app(server: &ServerSettings, cmd: &DestroyCommand) -> anyhow::Result<()> {
    let result = get(server, &format!("apps/destroy/{}", &cmd.app_name)).await?;
    let context: RunningAppContext =
        serde_json::from_value(result).context("Failed to parse context from API")?;
    
    let mut progress = ProgressTracker::new();
    progress.start_operation(&format!("Destroying app {}", &cmd.app_name))?;
    wait_for_task(server, &context).await?;
    progress.complete_operation(&format!("App {} destroyed", &cmd.app_name))?;
    
    Ok(())
}

pub async fn create_app(server: &ServerSettings, cmd: &CreateCommand) -> anyhow::Result<()> {
    let mut progress = ProgressTracker::new();

    // Step 1: Collect files
    progress.start_operation("Collecting files")?;
    let file_list = collect_files(&cmd.docker_compose_path)?;

    // Convert files for transport
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
    progress.complete_operation(&format!(
        "{} files collected",
        file_list.files.len().to_string().green()
    ))?;

    // Combine environment variables from env-file and command line
    let mut environment = cmd.env.clone();

    // Add environment variables from env-file if specified
    if let Some(env_file_path) = &cmd.env_file {
        progress.start_operation(&format!("Reading environment from {}", env_file_path))?;
        match parse_env_file(env_file_path) {
            Ok(env_file_vars) => {
                progress.complete_operation(&format!(
                    "{} environment variables loaded",
                    env_file_vars.len().to_string().green()
                ))?;
                environment.extend(env_file_vars);
            }
            Err(e) => {
                progress.fail_operation(&format!("Failed to parse env file: {}", e))?;
                return Err(anyhow::anyhow!("Failed to parse env file: {}", e));
            }
        }
    }

    // Prepare the payload
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

    // Serialize and send the payload
    let payload = serde_json::to_value(&payload).context("Failed to serialize payload")?;
    let size = scotty_core::utils::format::format_bytes(payload.to_string().len());

    progress.start_operation(&format!(
        "Beaming app {} to {} ({} payload)",
        &cmd.app_name.yellow(),
        &server.server.yellow(),
        size.blue()
    ))?;
    let result = get_or_post(server, "apps/create", "POST", Some(payload)).await?;
    progress.complete_operation(&format!(
        "Successfully beamed up app {}",
        &cmd.app_name.yellow()
    ))?;

    // Get the context and wait for the task to complete
    let context: RunningAppContext =
        serde_json::from_value(result).context("Failed to parse context from API")?;

    // Wait for deployment to complete
    wait_for_task(server, &context).await?;

    // Get app details
    progress.start_operation("Retrieving application details")?;
    let app_data = get_app_info(server, &context.app_data.name).await?;
    progress.complete_operation("Application details retrieved")?;

    progress_println!(
        progress,
        "\n✅ Deployment of {} complete!\n",
        &cmd.app_name.green().style(OwoStyle::new().bold())
    );
    print_app_info(&app_data)?;
    Ok(())
}
