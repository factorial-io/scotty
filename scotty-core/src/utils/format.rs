use jiff::SignedDuration;

pub fn format_duration(duration: &SignedDuration) -> String {
    let is_negative = duration.is_negative();
    let secs = duration.as_secs().unsigned_abs();

    let days = secs / 86_400;
    let hours = secs / 3_600 % 24;
    let minutes = secs / 60 % 60;
    let seconds = secs % 60;

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

    // Same output as the chrono-based version: the sign comes from the full
    // duration, the digits from whole seconds.
    #[test]
    fn format_duration_edge_cases() {
        let f = |d: SignedDuration| format_duration(&d);
        assert_eq!(f(SignedDuration::ZERO), "00m 00s");
        assert_eq!(f(SignedDuration::from_millis(500)), "00m 00s");
        assert_eq!(f(SignedDuration::from_millis(-500)), "-00m 00s");
        assert_eq!(f(SignedDuration::from_secs(-90)), "-01m 30s");
        assert_eq!(f(SignedDuration::from_secs(3_725)), "01h 02m 05s");
        assert_eq!(f(SignedDuration::from_secs(90_061)), "01d 01h 01m 01s");
    }

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
