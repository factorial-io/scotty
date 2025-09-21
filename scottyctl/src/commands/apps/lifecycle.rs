use anyhow::Context;
use owo_colors::OwoColorize;

use crate::{
    api::get,
    cli::{AdoptCommand, DestroyCommand, PurgeCommand, RebuildCommand, RunCommand, StopCommand},
    context::AppContext,
};
use scotty_core::{apps::app_data::AppData, tasks::running_app_context::RunningAppContext};

use super::{call_apps_api, format_app_info};

/// Rebuild an app
pub async fn rebuild_app(context: &AppContext, cmd: &RebuildCommand) -> anyhow::Result<()> {
    call_apps_api(context, "rebuild", &cmd.app_name).await
}

/// Run/start an app
pub async fn run_app(context: &AppContext, cmd: &RunCommand) -> anyhow::Result<()> {
    call_apps_api(context, "run", &cmd.app_name).await
}

/// Stop an app
pub async fn stop_app(context: &AppContext, cmd: &StopCommand) -> anyhow::Result<()> {
    call_apps_api(context, "stop", &cmd.app_name).await
}

/// Purge an app
pub async fn purge_app(context: &AppContext, cmd: &PurgeCommand) -> anyhow::Result<()> {
    call_apps_api(context, "purge", &cmd.app_name).await
}

/// Adopt an existing app into Scotty management
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

/// Destroy an app
pub async fn destroy_app(context: &AppContext, cmd: &DestroyCommand) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!("Destroying app {}...", &cmd.app_name.yellow()));
    ui.run(async || {
        let result = get(context.server(), &format!("apps/destroy/{}", &cmd.app_name)).await?;
        let app_context: RunningAppContext =
            serde_json::from_value(result).context("Failed to parse context from API")?;
        crate::api::wait_for_task(context.server(), &app_context, ui).await?;
        ui.success(format!(
            "App {} destroyed successfully",
            &cmd.app_name.yellow()
        ));

        Ok(format!("App {} destroyed", &cmd.app_name))
    })
    .await
}
