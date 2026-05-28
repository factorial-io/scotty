use chrono::TimeDelta;
use scotty_core::{apps::app_data::AppStatus, utils::format::format_chrono_duration};
use serde_json::Value;

/// Format the details of a custom action (everything below the title line) as an
/// indented, human-readable block. Shared by `action:get` and
/// `admin:actions:get` so the two stay consistent as the model evolves.
pub fn format_custom_action_details(action: &Value) -> String {
    use owo_colors::OwoColorize;

    let field =
        |label: &str, key: &str| format!("  {label}{}\n", action[key].as_str().unwrap_or(""));

    let mut output = String::new();
    output += &field("Name:        ", "name");
    output += &field("Description: ", "description");
    output += &field("Status:      ", "status");
    output += &field("Permission:  ", "permission");
    output += &field("Created By:  ", "created_by");
    output += &field("Created At:  ", "created_at");

    if let Some(reviewed_by) = action["reviewed_by"].as_str() {
        output += &format!("  Reviewed By: {reviewed_by}\n");
    }
    if let Some(reviewed_at) = action["reviewed_at"].as_str() {
        output += &format!("  Reviewed At: {reviewed_at}\n");
    }
    if let Some(comment) = action["review_comment"].as_str() {
        output += &format!("  Comment:     {comment}\n");
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
    output
}

pub fn format_since(duration: &Option<TimeDelta>) -> String {
    match duration {
        Some(d) => format_chrono_duration(d),
        None => "N/A".to_string(),
    }
}

pub fn colored_by_status(name: &str, status: &AppStatus) -> String {
    use owo_colors::OwoColorize;
    match status {
        AppStatus::Starting | AppStatus::Running => name.green().to_string(),
        AppStatus::Stopped => name.blue().to_string(),
        AppStatus::Creating => name.bright_green().to_string(),
        AppStatus::Destroying => name.bright_red().to_string(),
        AppStatus::Unsupported => name.white().to_string(),
    }
}
