#![allow(dead_code)]
use chrono::TimeDelta;

pub fn format_chrono_duration(duration: &TimeDelta) -> String {
    let is_negative = duration < &TimeDelta::zero();
    let duration = duration.abs();

    let days = duration.num_days();
    let hours = duration.num_hours() % 24;
    let minutes = duration.num_minutes() % 60;
    let seconds = duration.num_seconds() % 60;

    let mut segments = vec![];
    if days > 0 {
        segments.push(format!("{:02}d", days));
    }
    if days > 0 || hours > 0 {
        segments.push(format!("{:02}h", hours));
    }
    segments.push(format!("{:02}m", minutes));
    segments.push(format!("{:02}s", seconds));
    let formatted = format!(
        "{}{}",
        if is_negative { "-" } else { "" },
        segments.join(" ")
    );

    formatted
}

pub fn format_bytes(bytes: usize) -> String {
    let kb = bytes as f64 / 1024.0;
    let mb = kb / 1024.0;
    let gb = mb / 1024.0;

    if gb >= 1f64 {
        format!("{:.2} GB", gb)
    } else if mb >= 1f64 {
        format!("{:.2} MB", mb)
    } else if kb >= 1f64 {
        format!("{:.2} KB", kb)
    } else {
        format!("{} B", bytes)
    }
}
