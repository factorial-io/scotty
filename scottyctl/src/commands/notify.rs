use anyhow::Context;

use crate::{
    api::get_or_post,
    cli::{NotifyAddCommand, NotifyRemoveCommand},
    commands::apps::format_app_info,
    utils::ui::Ui,
    ServerSettings,
};
use scotty_core::{apps::app_data::AppData, notification_types::RemoveNotificationRequest};

pub async fn add_notification(
    server: &ServerSettings,
    cmd: &NotifyAddCommand,
) -> anyhow::Result<()> {
    let ui = Ui::new();
    ui.new_status_line("Adding notification...");
    ui.run(async || {
        let payload = serde_json::json!({
            "app_name": cmd.app_name,
            "service_ids": cmd.service_id,
        });

        let result = get_or_post(server, "apps/notify/add", "POST", Some(payload)).await?;

        let app_data: AppData =
            serde_json::from_value(result).context("Failed to parse context from API")?;

        ui.success(format!(
            "Notification added successfully to app {}",
            app_data.name
        ));

        format_app_info(&app_data)
    })
    .await
}

pub async fn remove_notification(
    server: &ServerSettings,
    cmd: &NotifyRemoveCommand,
) -> anyhow::Result<()> {
    let ui = Ui::new();
    ui.new_status_line("Removing notification...");
    ui.run(async || {
        let payload = RemoveNotificationRequest {
            app_name: cmd.app_name.clone(),
            service_ids: cmd.service_id.clone(),
        };

        let payload = serde_json::to_value(&payload).context("Failed to serialize payload")?;
        let result = get_or_post(server, "apps/notify/remove", "POST", Some(payload)).await?;

        let app_data: AppData =
            serde_json::from_value(result).context("Failed to parse context from API")?;

        ui.success(format!(
            "Notification removed successfully from app {}",
            app_data.name
        ));

        format_app_info(&app_data)
    })
    .await
}
