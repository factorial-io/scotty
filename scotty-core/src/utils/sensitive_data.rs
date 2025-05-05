//! Utilities for handling sensitive data
use std::collections::HashMap;
use url::Url;

/// Patterns that identify sensitive environment variables
pub const SENSITIVE_PATTERNS: [&str; 10] = [
    "password",
    "secret",
    "token",
    "key",
    "auth",
    "credential",
    "cert",
    "private",
    "api_key",
    "pass",
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

/// Check if a value appears to be a URI string that might contain credentials
///
/// # Arguments
/// * `value` - The string to check
///
/// # Returns
/// `true` if the value is likely a URI with credentials, `false` otherwise
pub fn is_uri_with_possible_credentials(value: &str) -> bool {
    // Try to parse as URL first
    if let Ok(url) = Url::parse(value) {
        // Check if URL has a username and password component
        if let Some(password) = url.password() {
            return !password.is_empty();
        }
    }

    false
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

/// Mask credentials in a URI string using the url crate for parsing
///
/// # Arguments
/// * `uri` - The URI string that might contain credentials
///
/// # Returns
/// A URI with credentials masked, if present
pub fn mask_uri_credentials(uri: &str) -> String {
    let mut result = uri.to_string();
    // Try to parse the URI
    if let Ok(mut parsed_url) = Url::parse(uri) {
        // Check if URL has a password component
        if let Some(password) = parsed_url.password() {
            if !password.is_empty() {
                // Create masked password
                let masked_password = mask_sensitive_value(password);

                // Set the masked password back into the URL
                // The unwrap is safe here since we've already verified the URL is valid
                let _ = parsed_url.set_password(Some(&masked_password));

                // Return the URL with masked password
                result = parsed_url.to_string();
            }
        }
    }
    result
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
                // If key is sensitive, mask the entire value
                (k.clone(), mask_sensitive_value(v))
            } else if is_uri_with_possible_credentials(v) {
                // If value is a URI with credentials, mask only the credentials
                (k.clone(), mask_uri_credentials(v))
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
    fn test_uri_with_credentials_detection() {
        // URLs with credentials
        assert!(is_uri_with_possible_credentials(
            "postgres://user:password@localhost/db"
        ));
        assert!(is_uri_with_possible_credentials(
            "mysql://admin:secret@dbserver.com:3306/mydb"
        ));
        assert!(is_uri_with_possible_credentials(
            "https://user:pass123@example.com"
        ));

        // URLs without credentials
        assert!(!is_uri_with_possible_credentials("postgres://localhost/db"));
        assert!(!is_uri_with_possible_credentials("https://example.com"));
        assert!(!is_uri_with_possible_credentials(
            "mysql://dbserver.com:3306/mydb"
        ));

        // Non-URL strings
        assert!(!is_uri_with_possible_credentials("not a url"));
        assert!(!is_uri_with_possible_credentials("password=secret"));
    }

    #[test]
    fn test_mask_uri_credentials() {
        // Test with standard database URLs
        assert_eq!(
            mask_uri_credentials("postgres://user:password@localhost/db"),
            "postgres://user:******rd@localhost/db"
        );

        assert_eq!(
            mask_uri_credentials("mysql://admin:secret123@dbserver.com:3306/mydb"),
            "mysql://admin:*******23@dbserver.com:3306/mydb"
        );

        // Test with HTTPS URLs
        assert_eq!(
            mask_uri_credentials("https://user:pass123@example.com"),
            "https://user:*****23@example.com/"
        );

        // URLs without credentials should remain unchanged (content-wise)
        let no_creds_url = "https://example.com";
        assert!(mask_uri_credentials(no_creds_url).contains("example.com"));

        // Invalid URLs should be returned as-is
        let invalid_url = "not:a:valid:url";
        assert_eq!(mask_uri_credentials(invalid_url), invalid_url);
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
        env_map.insert(
            "MONGODB_URI".to_string(),
            "mongodb://admin:mongodb_password@mongo.example.com:27017".to_string(),
        );

        let masked_map = mask_sensitive_env_map(&env_map);

        // Non-sensitive values should remain unchanged
        assert_eq!(masked_map.get("LOG_LEVEL"), Some(&"info".to_string()));
        assert_eq!(masked_map.get("PORT"), Some(&"8080".to_string()));

        // Sensitive keys should have their values masked
        assert_eq!(masked_map.get("API_KEY"), Some(&"*******23".to_string()));
        assert_eq!(masked_map.get("PASSWORD"), Some(&"******rd".to_string()));
        assert_eq!(
            masked_map.get("SECRET_TOKEN"),
            Some(&"***-*****-***-123".to_string())
        );

        // Database URLs should have only their passwords masked
        assert_eq!(
            masked_map.get("DATABASE_URL"),
            Some(&"postgres://user:**ss@localhost".to_string())
        );

        assert_eq!(
            masked_map.get("MONGODB_URI"),
            Some(&"mongodb://admin:************word@mongo.example.com:27017".to_string())
        );
    }
}
