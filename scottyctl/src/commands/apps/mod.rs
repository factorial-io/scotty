use anyhow::Context;
use owo_colors::OwoColorize;
use tabled::{builder::Builder, settings::Style};

use crate::{
    api::{get, wait_for_task},
    context::{AppContext, ServerSettings},
    utils::formatting::{colored_by_status, format_since},
};
use scotty_core::{
    apps::app_data::AppData,
    tasks::running_app_context::RunningAppContext,
};

// Re-export submodules
pub mod actions;
pub mod lifecycle;
pub mod list;
pub mod logs;
pub mod management;

// Re-export public functions to maintain backward compatibility
pub use actions::*;
pub use lifecycle::*;
pub use list::*;
pub use logs::*;
pub use management::*;

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
        let result = get(context.server(), &format!("apps/{verb}/{app_name}")).await?;
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

/// Shared utility for getting app information
pub async fn get_app_info(server: &ServerSettings, app_name: &str) -> anyhow::Result<AppData> {
    let app_data = get(server, &format!("apps/info/{app_name}")).await?;
    let app_data: AppData = serde_json::from_value(app_data).context("Failed to parse app data")?;

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
        result += format!("\n{table}").as_str();
    }

    Ok(result)
}