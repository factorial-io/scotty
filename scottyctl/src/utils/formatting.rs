use chrono::TimeDelta;
use scotty_core::{apps::app_data::AppStatus, utils::format::format_chrono_duration};

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
