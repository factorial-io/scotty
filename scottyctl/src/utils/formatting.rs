use jiff::{SignedDuration, Timestamp};
use scotty_core::{
    apps::app_data::{AppData, AppStatus, AppTtl},
    utils::format::format_duration,
};
use scotty_types::AppOperation;
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

pub fn format_since(duration: &Option<SignedDuration>) -> String {
    match duration {
        Some(d) => format_duration(d),
        None => "N/A".to_string(),
    }
}

/// One line for `app:info`: what runs on the app, since when, and its task.
pub fn format_operation(op: &AppOperation, now: Timestamp) -> String {
    let mut line = format!(
        "Operation: {} running since {}",
        op.operation,
        format_since(&Some(now.duration_since(op.started_at)))
    );
    if let Some(task_id) = op.task_id {
        line += &format!(" (task {task_id})");
    }
    line
}

/// The `Lifetime:` line of `app:info`: the TTL, whether the app is then
/// stopped or destroyed, and how long it has left.
pub fn format_lifetime(app: &AppData, now: Timestamp) -> String {
    let ttl = match app.get_ttl() {
        AppTtl::Forever => return "Lifetime: forever".into(),
        AppTtl::Days(d) => format!("{d}d"),
        AppTtl::Hours(h) => format!("{h}h"),
    };
    let end = if app.destroys_on_ttl() {
        "destroyed"
    } else {
        "stopped"
    };
    let left = match app.ttl_expires_at {
        None => "(not running)".to_string(),
        Some(at) if at <= now => "(due now)".to_string(),
        Some(at) => format!("in {}", format_duration(&at.duration_since(now))),
    };
    format!("Lifetime: {ttl}, then {end} {left}")
}

/// Status cell of `app:list`, marked with the running operation, if any.
pub fn status_with_operation(status: &AppStatus, operation: &Option<AppOperation>) -> String {
    match operation {
        Some(op) => format!("{status} [{}]", op.operation),
        None => status.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn op(task_id: Option<uuid::Uuid>) -> AppOperation {
        AppOperation {
            operation: "rebuild".into(),
            task_id,
            started_at: Timestamp::now() - SignedDuration::from_mins(3),
        }
    }

    #[test]
    fn operation_line_names_operation_and_task() {
        let id = uuid::Uuid::nil();
        let line = format_operation(&op(Some(id)), Timestamp::now());
        assert!(
            line.starts_with("Operation: rebuild running since "),
            "{line}"
        );
        assert!(line.ends_with(&format!("(task {id})")), "{line}");
        assert!(!format_operation(&op(None), Timestamp::now()).contains("task"));
    }

    #[test]
    fn lifetime_line() {
        use scotty_core::apps::app_data::AppSettings;
        let now = Timestamp::now();
        let app = |ttl, destroy, expires: Option<i64>| AppData {
            settings: Some(AppSettings {
                time_to_live: ttl,
                destroy_on_ttl: destroy,
                ..Default::default()
            }),
            ttl_expires_at: expires.map(|h| now + SignedDuration::from_hours(h)),
            ..Default::default()
        };
        assert_eq!(
            format_lifetime(&app(AppTtl::Days(14), true, Some(334)), now),
            "Lifetime: 14d, then destroyed in 13d 22h 00m 00s"
        );
        assert_eq!(
            format_lifetime(&app(AppTtl::Hours(12), false, None), now),
            "Lifetime: 12h, then stopped (not running)"
        );
        assert_eq!(
            format_lifetime(&app(AppTtl::Hours(12), false, Some(-1)), now),
            "Lifetime: 12h, then stopped (due now)"
        );
        assert_eq!(
            format_lifetime(&app(AppTtl::Forever, true, None), now),
            "Lifetime: forever"
        );
    }

    #[test]
    fn list_marks_busy_apps_only() {
        assert_eq!(
            status_with_operation(&AppStatus::Running, &Some(op(None))),
            "Running [rebuild]"
        );
        assert_eq!(status_with_operation(&AppStatus::Running, &None), "Running");
    }
}
