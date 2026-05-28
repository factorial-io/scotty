use anyhow::Context;
use owo_colors::OwoColorize;
use tabled::{builder::Builder, settings::Style};

use crate::{
    api::{get, wait_for_task},
    context::{AppContext, ServerSettings},
    utils::formatting::format_since,
};
use scotty_core::{apps::app_data::AppData, tasks::running_app_context::RunningAppContext};

// Re-export submodules
pub mod actions;
pub mod lifecycle;
pub mod list;
pub mod logs;
pub mod management;
pub mod shell;

// Re-export public functions to maintain backward compatibility
pub use actions::*;
pub use lifecycle::*;
pub use list::*;
pub use logs::*;
pub use management::*;
pub use shell::*;

/// Shared utility for calling apps API endpoints that return a RunningAppContext
pub async fn call_apps_api(context: &AppContext, verb: &str, app_name: &str) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Running action {} for app {} at {} ...",
        verb.yellow(),
        app_name.yellow(),
        context.server().server.yellow()
    ));
    ui.run(async || {
        // Connect WebSocket before starting the task for better real-time streaming
        let ws_connection =
            crate::websocket::AuthenticatedWebSocket::connect(context.server()).await;

        // Start the task
        let result = get(context.server(), &format!("apps/{verb}/{app_name}")).await?;
        let app_context: RunningAppContext =
            serde_json::from_value(result).context("Failed to parse context from API")?;

        // Wait for task with pre-connected WebSocket
        wait_for_task(context.server(), &app_context, ui, ws_connection).await?;
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

/// Shared utility for getting app information
pub async fn get_app_info(server: &ServerSettings, app_name: &str) -> anyhow::Result<AppData> {
    let app_data = get(server, &format!("apps/info/{app_name}")).await?;
    let app_data: AppData = serde_json::from_value(app_data).context("Failed to parse app data")?;

    Ok(app_data)
}

/// Validate that an app and service exist, showing helpful error if not
pub async fn validate_app_and_service(
    context: &AppContext,
    app_name: &str,
    service_name: &str,
    command_name: &str,
) -> anyhow::Result<AppData> {
    use owo_colors::OwoColorize;

    let ui = context.ui();

    // First validate that the app and service exist
    ui.new_status_line(format!(
        "Validating app {} and service {}...",
        app_name.yellow(),
        service_name.yellow()
    ));

    // Get app info and validate service exists
    let app_data = match get_app_info(context.server(), app_name).await {
        Ok(data) => data,
        Err(e) => {
            ui.failed(format!("Failed to get app information: {}", e));
            return Err(e);
        }
    };

    // Check if the requested service exists
    let available_services: Vec<String> = app_data
        .services
        .iter()
        .map(|s| s.service.clone())
        .collect();

    if !available_services.contains(&service_name.to_string()) {
        ui.failed(format!(
            "Service '{}' not found in app '{}'",
            service_name.red(),
            app_name.yellow()
        ));

        // Show available services in a nice format
        ui.println("");
        ui.println(format!("Available services in {}:", app_name.yellow()));

        for service in &app_data.services {
            let status_icon = if service.is_running() {
                "✓".green().to_string()
            } else {
                "✗".red().to_string()
            };
            ui.println(format!(
                "  {} {} ({})",
                status_icon,
                service.service.green(),
                service.status.to_string().dimmed()
            ));
        }

        ui.println("");
        ui.println(format!(
            "Usage: {} {} {} <service_name>",
            "scottyctl".cyan(),
            command_name,
            app_name
        ));

        return Err(anyhow::anyhow!(
            "Service '{}' not found. Please choose from the available services listed above.",
            service_name
        ));
    }

    ui.success(format!(
        "Found service {} in app {}",
        service_name.yellow(),
        app_name.yellow()
    ));

    Ok(app_data)
}

/// Shared utility for formatting app information into a table
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

    if let Some(settings) = &app_data.settings {
        if !settings.notify.is_empty() {
            result += "\nNotification services";
            let mut builder = Builder::default();
            builder.push_record(["Type", "Service-Id", "Context"]);
            for notification in &settings.notify {
                #[allow(unused_assignments)]
                let mut context: String = "".into();
                builder.push_record(match notification {
                    scotty_core::notification_types::NotificationReceiver::Log => {
                        ["Log", "Log", ""]
                    }
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
            result += format!("\n{table}").as_str();
        }
    }

    // Add scope information if available
    if let Some(settings) = &app_data.settings {
        if !settings.scopes.is_empty() {
            result += &format!("\nScopes: {}", settings.scopes.join(", "));
        }
    }

    Ok(result)
}
