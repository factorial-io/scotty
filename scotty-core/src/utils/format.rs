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

/// Sanitizes a string to be used as an environment variable name.
/// Replaces hyphens and periods with underscores and converts to uppercase.
///
/// Docker Compose service names can contain [a-zA-Z0-9\._\-] characters,
/// but environment variable names can only contain [a-zA-Z0-9_].
///
/// # Examples
///
/// ```
/// use scotty_core::utils::format::sanitize_env_var_name;
///
/// assert_eq!(sanitize_env_var_name("my-service"), "MY_SERVICE");
/// assert_eq!(sanitize_env_var_name("my.service"), "MY_SERVICE");
/// assert_eq!(sanitize_env_var_name("my-service.v2"), "MY_SERVICE_V2");
/// assert_eq!(sanitize_env_var_name("simple_service"), "SIMPLE_SERVICE");
/// assert_eq!(sanitize_env_var_name("web"), "WEB");
/// ```
pub fn sanitize_env_var_name(name: &str) -> String {
    name.replace(['-', '.'], "_").to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_env_var_name() {
        assert_eq!(sanitize_env_var_name("my-service"), "MY_SERVICE");
        assert_eq!(sanitize_env_var_name("my.service"), "MY_SERVICE");
        assert_eq!(sanitize_env_var_name("my-service.v2"), "MY_SERVICE_V2");
        assert_eq!(sanitize_env_var_name("simple_service"), "SIMPLE_SERVICE");
        assert_eq!(sanitize_env_var_name("web"), "WEB");
        assert_eq!(
            sanitize_env_var_name("multi-word-service"),
            "MULTI_WORD_SERVICE"
        );
        assert_eq!(
            sanitize_env_var_name("service-with-many-hyphens"),
            "SERVICE_WITH_MANY_HYPHENS"
        );
        assert_eq!(
            sanitize_env_var_name("service.with.dots"),
            "SERVICE_WITH_DOTS"
        );
        assert_eq!(
            sanitize_env_var_name("mixed-service.v1_test"),
            "MIXED_SERVICE_V1_TEST"
        );
    }
}
