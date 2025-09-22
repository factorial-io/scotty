use anyhow::Context;
use owo_colors::OwoColorize;
use tabled::{builder::Builder, settings::Style};

use crate::{
    api::get,
    cli::InfoCommand,
    context::AppContext,
    utils::formatting::{colored_by_status, format_since},
};
use scotty_core::apps::{app_data::AppData, shared_app_list::AppDataVec};

use super::format_app_info;

/// List all installed apps
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

/// Get info for a specific app
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
