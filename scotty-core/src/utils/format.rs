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
        segments.push(format!("{days:02}d"));
    }
    if days > 0 || hours > 0 {
        segments.push(format!("{hours:02}h"));
    }
    segments.push(format!("{minutes:02}m"));
    segments.push(format!("{seconds:02}s"));
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
        format!("{gb:.2} GB")
    } else if mb >= 1f64 {
        format!("{mb:.2} MB")
    } else if kb >= 1f64 {
        format!("{kb:.2} KB")
    } else {
        format!("{bytes} B")
    }
}
