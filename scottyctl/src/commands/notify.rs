use anyhow::Context;

use crate::{
    api::get_or_post,
    cli::{NotifyAddCommand, NotifyRemoveCommand},
    commands::apps::format_app_info,
    utils::ui::StatusLine,
    ServerSettings,
};
use scotty_core::{apps::app_data::AppData, notification_types::RemoveNotificationRequest};

pub async fn add_notification(
    server: &ServerSettings,
    cmd: &NotifyAddCommand,
) -> anyhow::Result<()> {
    let status_line = StatusLine::new("Adding notification...");
    status_line
        .run(async || {
            let payload = serde_json::json!({
                "app_name": cmd.app_name,
                "service_ids": cmd.service_id,
            });

            let result = get_or_post(server, "apps/notify/add", "POST", Some(payload)).await?;

            let app_data: AppData =
                serde_json::from_value(result).context("Failed to parse context from API")?;

            status_line.success(format!(
                "Notification added successfully to app {}",
                app_data.name
            ));

            Ok(format_app_info(&app_data)?)
        })
        .await
}

pub async fn remove_notification(
    server: &ServerSettings,
    cmd: &NotifyRemoveCommand,
) -> anyhow::Result<()> {
    let status_line = StatusLine::new("Removing notification...");
    status_line
        .run(async || {
            let payload = RemoveNotificationRequest {
                app_name: cmd.app_name.clone(),
                service_ids: cmd.service_id.clone(),
            };

            let payload = serde_json::to_value(&payload).context("Failed to serialize payload")?;
            let result = get_or_post(server, "apps/notify/remove", "POST", Some(payload)).await?;

            let app_data: AppData =
                serde_json::from_value(result).context("Failed to parse context from API")?;

            status_line.success(format!(
                "Notification removed successfully from app {}",
                app_data.name
            ));

            Ok(format_app_info(&app_data)?)
        })
        .await
}
