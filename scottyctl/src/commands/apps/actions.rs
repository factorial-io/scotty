use anyhow::Context;
use owo_colors::OwoColorize;
use serde_json::json;
use std::collections::HashMap;
use tabled::{builder::Builder, settings::Style};

use crate::{
    api::{delete, get, get_or_post, post, wait_for_task},
    cli::{
        ActionCommand, ActionCreateCommand, ActionDeleteCommand, ActionGetCommand,
        ActionListCommand,
    },
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
        // Connect WebSocket before starting the task
        let ws_connection =
            crate::websocket::AuthenticatedWebSocket::connect(context.server()).await;

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

        wait_for_task(context.server(), &app_context, ui, ws_connection).await?;

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

/// List custom actions for an app
pub async fn list_custom_actions(
    context: &AppContext,
    cmd: &ActionListCommand,
) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Getting custom actions for app {}...",
        &cmd.app_name.yellow()
    ));
    ui.run(async || {
        let result = get(
            context.server(),
            &format!("apps/{}/custom-actions", &cmd.app_name),
        )
        .await?;

        let response: serde_json::Value = result;
        let actions = response["actions"]
            .as_array()
            .context("Failed to parse actions list")?;

        if actions.is_empty() {
            return Ok(format!(
                "No custom actions found for app '{}'.",
                cmd.app_name
            ));
        }

        let mut builder = Builder::default();
        builder.push_record(vec![
            "Name",
            "Description",
            "Status",
            "Permission",
            "Created By",
        ]);

        for action in actions {
            builder.push_record(vec![
                action["name"].as_str().unwrap_or(""),
                action["description"].as_str().unwrap_or(""),
                action["status"].as_str().unwrap_or(""),
                action["permission"].as_str().unwrap_or(""),
                action["created_by"].as_str().unwrap_or(""),
            ]);
        }

        let mut table = builder.build();
        table.with(Style::rounded());
        ui.success(format!(
            "Custom actions for app '{}':",
            cmd.app_name.yellow()
        ));
        Ok(table.to_string())
    })
    .await
}

/// Get details of a custom action
pub async fn get_custom_action(context: &AppContext, cmd: &ActionGetCommand) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Getting custom action '{}' for app '{}'...",
        &cmd.action_name.yellow(),
        &cmd.app_name.yellow()
    ));
    ui.run(async || {
        let result = get(
            context.server(),
            &format!("apps/{}/custom-actions/{}", &cmd.app_name, &cmd.action_name),
        )
        .await?;

        let action: serde_json::Value = result;

        let mut output = format!(
            "Custom action '{}' for app '{}':\n\n",
            cmd.action_name.yellow(),
            cmd.app_name.yellow()
        );
        output += &format!("  Name:        {}\n", action["name"].as_str().unwrap_or(""));
        output += &format!(
            "  Description: {}\n",
            action["description"].as_str().unwrap_or("")
        );
        output += &format!(
            "  Status:      {}\n",
            action["status"].as_str().unwrap_or("")
        );
        output += &format!(
            "  Permission:  {}\n",
            action["permission"].as_str().unwrap_or("")
        );
        output += &format!(
            "  Created By:  {}\n",
            action["created_by"].as_str().unwrap_or("")
        );
        output += &format!(
            "  Created At:  {}\n",
            action["created_at"].as_str().unwrap_or("")
        );

        if let Some(reviewed_by) = action["reviewed_by"].as_str() {
            output += &format!("  Reviewed By: {}\n", reviewed_by);
        }
        if let Some(reviewed_at) = action["reviewed_at"].as_str() {
            output += &format!("  Reviewed At: {}\n", reviewed_at);
        }
        if let Some(comment) = action["review_comment"].as_str() {
            output += &format!("  Comment:     {}\n", comment);
        }

        output += "\n  Commands:\n";
        if let Some(commands) = action["commands"].as_object() {
            for (service, cmds) in commands {
                output += &format!("    {}:\n", service.bright_blue());
                if let Some(cmd_list) = cmds.as_array() {
                    for cmd in cmd_list {
                        output += &format!("      - {}\n", cmd.as_str().unwrap_or(""));
                    }
                }
            }
        }

        ui.success("Action details retrieved successfully!");
        Ok(output)
    })
    .await
}

/// Create a custom action for an app
pub async fn create_custom_action(
    context: &AppContext,
    cmd: &ActionCreateCommand,
) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Creating custom action '{}' for app '{}'...",
        &cmd.action_name.yellow(),
        &cmd.app_name.yellow()
    ));

    // Parse commands from SERVICE:COMMAND format
    let mut commands: HashMap<String, Vec<String>> = HashMap::new();
    for cmd_str in &cmd.commands {
        let parts: Vec<&str> = cmd_str.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid command format '{}'. Expected SERVICE:COMMAND",
                cmd_str
            ));
        }
        let service = parts[0].to_string();
        let command = parts[1].to_string();
        commands.entry(service).or_default().push(command);
    }

    if commands.is_empty() {
        return Err(anyhow::anyhow!(
            "At least one command is required. Use --command SERVICE:COMMAND"
        ));
    }

    let payload = json!({
        "name": cmd.action_name,
        "description": cmd.description,
        "permission": cmd.permission,
        "commands": commands
    });

    let result = post(
        context.server(),
        &format!("apps/{}/custom-actions", &cmd.app_name),
        payload,
    )
    .await?;

    let status = result["status"].as_str().unwrap_or("unknown");

    if status == "pending" {
        ui.success(format!(
            "Custom action '{}' created for app '{}'. Status: {} (awaiting approval)",
            cmd.action_name.yellow(),
            cmd.app_name.yellow(),
            status.bright_yellow()
        ));
    } else {
        ui.success(format!(
            "Custom action '{}' created for app '{}'. Status: {}",
            cmd.action_name.yellow(),
            cmd.app_name.yellow(),
            status.bright_green()
        ));
    }
    Ok(())
}

/// Delete a custom action from an app
pub async fn delete_custom_action(
    context: &AppContext,
    cmd: &ActionDeleteCommand,
) -> anyhow::Result<()> {
    let ui = context.ui();
    ui.new_status_line(format!(
        "Deleting custom action '{}' from app '{}'...",
        &cmd.action_name.yellow(),
        &cmd.app_name.yellow()
    ));

    delete(
        context.server(),
        &format!("apps/{}/custom-actions/{}", &cmd.app_name, &cmd.action_name),
        None::<serde_json::Value>,
    )
    .await?;

    ui.success(format!(
        "Custom action '{}' deleted from app '{}'.",
        cmd.action_name.yellow(),
        cmd.app_name.yellow()
    ));
    Ok(())
}
