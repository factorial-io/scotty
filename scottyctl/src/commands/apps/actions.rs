use anyhow::Context;
use owo_colors::OwoColorize;

use crate::{
    api::{get_or_post, wait_for_task},
    cli::ActionCommand,
    context::AppContext,
};
use scotty_core::tasks::running_app_context::RunningAppContext;

use super::{format_app_info, get_app_info};

/// Run a custom action on an app
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
