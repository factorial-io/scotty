use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Creates a domain-safe version of an app name
///
/// DNS labels (parts between dots) are limited to 63 bytes.
/// This function ensures the app name will fit within this limit
/// while preserving as much of the original name as possible for readability.
///
/// If the name is already short enough, it returns it unchanged.
/// If it's too long, it truncates and appends a hash suffix.
///
/// # Arguments
///
/// * `app_name` - The original app name
///
/// # Returns
///
/// A string that is safe to use as a DNS label (max 63 bytes)
pub fn domain_safe_name(app_name: &str) -> String {
    const MAX_LABEL_LENGTH: usize = 63;

    // If the app name is already within limits, return it as is
    if app_name.len() <= MAX_LABEL_LENGTH {
        return app_name.to_string();
    }

    // We need to create a shortened version with a hash suffix
    // Format: [truncated-name]-[hash]
    // The hash will be 8 hex characters (4 bytes)
    // Allow 9 characters for "-" plus the hash
    const HASH_PART_LENGTH: usize = 9;
    let max_name_part_length = MAX_LABEL_LENGTH - HASH_PART_LENGTH;

    // Calculate hash of the full name
    let mut hasher = DefaultHasher::new();
    app_name.hash(&mut hasher);
    let hash = hasher.finish();

    // Truncate the name to fit within byte limits
    // We need to be careful with Unicode boundaries
    let mut name_part = String::new();
    let mut byte_count = 0;

    for c in app_name.chars() {
        let c_len = c.len_utf8();
        if byte_count + c_len > max_name_part_length {
            break;
        }
        name_part.push(c);
        byte_count += c_len;
    }

    // Combine the shortened name with a hash suffix
    format!("{}-{:x}", name_part, hash & 0xFFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_name_unchanged() {
        let name = "short-app-name";
        assert_eq!(domain_safe_name(name), name);
    }

    #[test]
    fn test_long_name_is_hashed() {
        let long_name = "this-is-an-extremely-long-application-name-that-would-definitely-exceed-the-dns-label-length-limit";
        let result = domain_safe_name(long_name);

        // Verify the result is not longer than the maximum allowed in bytes
        assert!(result.len() <= 63);

        // Verify the result starts with a portion of the original name
        assert!(result.starts_with("this-is-an-extremely-long-application-name"));

        // Verify the result contains a hash part
        assert!(result.contains('-'));
        let parts: Vec<&str> = result.rsplitn(2, '-').collect();
        assert_eq!(parts.len(), 2);

        // The last part should be a hex hash
        assert!(parts[0].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_consistent_hashing() {
        let name = "very-long-application-name-that-needs-to-be-hashed-for-dns-compatibility";
        let result1 = domain_safe_name(name);
        let result2 = domain_safe_name(name);

        // Same input should produce same output
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_exactly_at_limit() {
        let name = "a".repeat(63);
        assert_eq!(domain_safe_name(&name), name);

        let name = "a".repeat(64);
        assert_ne!(domain_safe_name(&name), name);
        assert!(domain_safe_name(&name).len() <= 63);
    }

    #[test]
    fn test_unicode_handling() {
        // Unicode characters can take up multiple bytes
        let unicode_name = "app-with-unicode-日本語-한국어";
        let result = domain_safe_name(unicode_name);

        // Make sure the byte length is within the limit
        assert!(result.len() <= 63);

        // Test with a string that's exactly at the limit with unicode
        let long_unicode = "测试".repeat(21); // Each character is 3 bytes
        assert!(long_unicode.len() > 63);
        let result = domain_safe_name(&long_unicode);
        assert!(result.len() <= 63);
    }
}
