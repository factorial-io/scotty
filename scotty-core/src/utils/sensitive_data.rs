//! Utilities for handling sensitive data
use std::collections::HashMap;

/// Patterns that identify sensitive environment variables
pub const SENSITIVE_PATTERNS: [&str; 9] = [
    "password",
    "secret",
    "token",
    "key",
    "auth",
    "credential",
    "cert",
    "private",
    "api_key",
];

/// Check if a key represents sensitive data
///
/// # Arguments
/// * `key` - The key name to check
///
/// # Returns
/// `true` if the key is considered sensitive, `false` otherwise
pub fn is_sensitive(key: &str) -> bool {
    let lowercase_key = key.to_lowercase();
    SENSITIVE_PATTERNS
        .iter()
        .any(|pattern| lowercase_key.contains(pattern))
}

/// Mask sensitive data with asterisks while preserving some information
///
/// # Arguments
/// * `value` - The sensitive value to mask
///
/// # Returns
/// A masked version of the value according to these rules:
/// - For values < 12 chars: Last 2 chars visible, rest masked
/// - For values >= 12 chars: Last 4 chars visible, rest masked
/// - Dashes ('-') are preserved in their original positions
pub fn mask_sensitive_value(value: &str) -> String {
    let value_len = value.len();
    let visible_suffix_len = if value_len >= 12 { 4 } else { 2.min(value_len) };

    // Create masked version
    let mut masked = String::with_capacity(value_len);
    let prefix_len = value_len.saturating_sub(visible_suffix_len);

    // Add masked prefix, preserving any "-" characters
    for (i, c) in value.chars().enumerate() {
        if i < prefix_len {
            masked.push(if c == '-' { '-' } else { '*' });
        } else {
            // Add visible suffix
            masked.push(c);
        }
    }

    masked
}

/// Creates a new HashMap with sensitive values masked
///
/// # Arguments
/// * `env_map` - A HashMap containing environment variables
///
/// # Returns
/// A new HashMap with the same keys, but sensitive values are masked
pub fn mask_sensitive_env_map(env_map: &HashMap<String, String>) -> HashMap<String, String> {
    env_map
        .iter()
        .map(|(k, v)| {
            if is_sensitive(k) {
                (k.clone(), mask_sensitive_value(v))
            } else {
                (k.clone(), v.clone())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_sensitive() {
        // Positive cases
        assert!(is_sensitive("password"));
        assert!(is_sensitive("PASSWORD"));
        assert!(is_sensitive("db_password"));
        assert!(is_sensitive("api_key"));
        assert!(is_sensitive("AUTH_TOKEN"));
        assert!(is_sensitive("my_secret_value"));
        assert!(is_sensitive("aws_secret_access_key"));
        assert!(is_sensitive("private_key"));
        assert!(is_sensitive("client_credential"));
        assert!(is_sensitive("certificate_key"));

        // Negative cases
        assert!(!is_sensitive("username"));
        assert!(!is_sensitive("database_url"));
        assert!(!is_sensitive("host"));
        assert!(!is_sensitive("port"));
        assert!(!is_sensitive("region"));
        assert!(!is_sensitive("log_level"));
    }

    #[test]
    fn test_mask_sensitive_value() {
        // Short values (< 12 chars)
        assert_eq!(mask_sensitive_value("pass123"), "*****23");
        assert_eq!(mask_sensitive_value("key"), "*ey");
        assert_eq!(mask_sensitive_value("ab"), "ab");
        assert_eq!(mask_sensitive_value("a"), "a");
        assert_eq!(mask_sensitive_value(""), "");

        // Values with exactly 12 chars
        assert_eq!(mask_sensitive_value("password1234"), "********1234");

        // Long values (>= 12 chars)
        assert_eq!(
            mask_sensitive_value("my-super-secret-token"),
            "**-*****-******-*oken"
        );
        assert_eq!(
            mask_sensitive_value("api_key_12345678901234"),
            "******************1234"
        );

        // With dashes
        assert_eq!(mask_sensitive_value("jwt-token-xyz"), "***-*****-xyz");
        assert_eq!(mask_sensitive_value("a-b-c"), "*-*-c");
    }

    #[test]
    fn test_mask_sensitive_env_map() {
        let mut env_map = HashMap::new();
        env_map.insert(
            "DATABASE_URL".to_string(),
            "postgres://user:pass@localhost".to_string(),
        );
        env_map.insert("API_KEY".to_string(), "secret123".to_string());
        env_map.insert("PASSWORD".to_string(), "p@ssw0rd".to_string());
        env_map.insert("LOG_LEVEL".to_string(), "info".to_string());
        env_map.insert("PORT".to_string(), "8080".to_string());
        env_map.insert("SECRET_TOKEN".to_string(), "jwt-token-xyz-123".to_string());

        let masked_map = mask_sensitive_env_map(&env_map);

        // Non-sensitive values should remain unchanged
        assert_eq!(masked_map.get("LOG_LEVEL"), Some(&"info".to_string()));
        assert_eq!(masked_map.get("PORT"), Some(&"8080".to_string()));
        assert_eq!(
            masked_map.get("DATABASE_URL"),
            Some(&"postgres://user:pass@localhost".to_string())
        );

        // Sensitive values should be masked
        assert_eq!(masked_map.get("API_KEY"), Some(&"*******23".to_string()));
        assert_eq!(masked_map.get("PASSWORD"), Some(&"******rd".to_string()));
        assert_eq!(
            masked_map.get("SECRET_TOKEN"),
            Some(&"***-*****-***-123".to_string())
        );
    }
}
