//! Secure secret handling using the secrecy crate with custom partial masking
//!
//! This module provides `MaskedSecret` and `SecretHashMap` types that:
//! - Protect secrets in memory (zeroized on drop)
//! - Show partial masking in Debug output (e.g., "****1234")
//! - Serialize full values for YAML files (docker-compose)
//! - Require explicit `.expose_secret()` calls for access

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

use super::sensitive_data::{is_sensitive, mask_sensitive_value};

/// A secret string that provides memory protection via the secrecy crate
/// while showing partial masking in Debug output for usability.
///
/// # Example
/// ```
/// use scotty_core::utils::secret::MaskedSecret;
///
/// let password = MaskedSecret::new("super-secret-password-123".to_string());
///
/// // Debug shows masked value (preserves dashes)
/// assert_eq!(format!("{:?}", password), "\"*****-******-********-123\"");
///
/// // Explicit access required
/// assert_eq!(password.expose_secret(), "super-secret-password-123");
/// ```
#[derive(Clone)]
pub struct MaskedSecret(SecretString);

impl MaskedSecret {
    /// Create a new masked secret from a String
    pub fn new(value: String) -> Self {
        Self(SecretString::new(value.into_boxed_str()))
    }

    /// Create a new masked secret from a string slice
    pub fn from_str(s: &str) -> Self {
        Self::new(s.to_string())
    }

    /// Expose the secret value - this should only be called where absolutely necessary
    /// (e.g., passing to docker commands, writing to YAML files)
    pub fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }
}

impl std::fmt::Debug for MaskedSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let masked = mask_sensitive_value(self.0.expose_secret());
        write!(f, "\"{}\"", masked)
    }
}

impl std::fmt::Display for MaskedSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let masked = mask_sensitive_value(self.0.expose_secret());
        write!(f, "{}", masked)
    }
}

impl Serialize for MaskedSecret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize the full value (needed for docker-compose.override.yml)
        self.0.expose_secret().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MaskedSecret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(MaskedSecret::new(value))
    }
}

impl PartialEq for MaskedSecret {
    fn eq(&self, other: &Self) -> bool {
        self.0.expose_secret() == other.0.expose_secret()
    }
}

impl Eq for MaskedSecret {}

/// A HashMap containing MaskedSecret values with smart Debug output
///
/// # Example
/// ```
/// use scotty_core::utils::secret::SecretHashMap;
///
/// let mut env = SecretHashMap::new();
/// env.insert("DATABASE_PASSWORD".to_string(), "super-secret-123".to_string());
/// env.insert("LOG_LEVEL".to_string(), "info".to_string());
///
/// // Debug shows masked values for sensitive keys
/// println!("{:?}", env); // Shows masked password
/// ```
#[derive(Clone)]
pub struct SecretHashMap {
    map: HashMap<String, MaskedSecret>,
}

impl SecretHashMap {
    /// Create a new empty SecretHashMap
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Create from a plain HashMap, wrapping all values in MaskedSecret
    pub fn from_hashmap(map: HashMap<String, String>) -> Self {
        Self {
            map: map
                .into_iter()
                .map(|(k, v)| (k, MaskedSecret::new(v)))
                .collect(),
        }
    }

    /// Insert a key-value pair
    pub fn insert(&mut self, key: String, value: String) {
        self.map.insert(key, MaskedSecret::new(value));
    }

    /// Get a reference to a secret value
    pub fn get(&self, key: &str) -> Option<&MaskedSecret> {
        self.map.get(key)
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Check if the map is empty
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Iterate over key-value pairs
    pub fn iter(&self) -> impl Iterator<Item = (&String, &MaskedSecret)> {
        self.map.iter()
    }

    /// Convert to a plain HashMap - only use when secrets must be exposed
    /// (e.g., for docker-compose env vars, process execution)
    pub fn expose_all(&self) -> HashMap<String, String> {
        self.map
            .iter()
            .map(|(k, v)| (k.clone(), v.expose_secret().to_string()))
            .collect()
    }

    /// Convert to a HashMap with masked values for API responses
    pub fn to_masked_hashmap(&self) -> HashMap<String, String> {
        self.map
            .iter()
            .map(|(k, v)| {
                if is_sensitive(k) {
                    (k.clone(), mask_sensitive_value(v.expose_secret()))
                } else {
                    (k.clone(), v.expose_secret().to_string())
                }
            })
            .collect()
    }
}

impl Default for SecretHashMap {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SecretHashMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_map = f.debug_map();
        for (key, value) in &self.map {
            debug_map.entry(key, value); // Uses MaskedSecret's Debug
        }
        debug_map.finish()
    }
}

impl Serialize for SecretHashMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as plain HashMap with full values (for YAML files)
        self.expose_all().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SecretHashMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let map = HashMap::<String, String>::deserialize(deserializer)?;
        Ok(SecretHashMap::from_hashmap(map))
    }
}

impl PartialEq for SecretHashMap {
    fn eq(&self, other: &Self) -> bool {
        if self.map.len() != other.map.len() {
            return false;
        }
        self.map
            .iter()
            .all(|(k, v)| other.map.get(k).map(|ov| v == ov).unwrap_or(false))
    }
}

impl Eq for SecretHashMap {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_masked_secret_debug_shows_partial() {
        let secret = MaskedSecret::new("super-secret-password-123".to_string());
        let debug_output = format!("{:?}", secret);

        // Should contain asterisks
        assert!(debug_output.contains("***"));

        // Should show last 4 chars (value is 26 chars, >= 12)
        assert!(debug_output.ends_with("123\""));

        // Should NOT show full value
        assert!(!debug_output.contains("super-secret-password"));
    }

    #[test]
    fn test_masked_secret_display_shows_partial() {
        let secret = MaskedSecret::new("api-key-12345".to_string());
        let display_output = format!("{}", secret);

        assert!(display_output.contains("***"));
        assert!(display_output.ends_with("345"));
    }

    #[test]
    fn test_masked_secret_expose_gives_full_value() {
        let secret = MaskedSecret::new("my-secret".to_string());
        assert_eq!(secret.expose_secret(), "my-secret");
    }

    #[test]
    fn test_masked_secret_short_value() {
        let secret = MaskedSecret::new("pass".to_string());
        let debug_output = format!("{:?}", secret);

        // Short values (< 12 chars) show last 2 chars
        assert!(debug_output.contains("**ss"));
    }

    #[test]
    fn test_masked_secret_serialization() {
        let secret = MaskedSecret::new("password123".to_string());
        let yaml = serde_norway::to_string(&secret).unwrap();

        // Should serialize full value for docker-compose
        assert_eq!(yaml.trim(), "password123");
    }

    #[test]
    fn test_masked_secret_deserialization() {
        let yaml = "my-password";
        let secret: MaskedSecret = serde_norway::from_str(yaml).unwrap();
        assert_eq!(secret.expose_secret(), "my-password");
    }

    #[test]
    fn test_secret_hashmap_creation() {
        let mut map = HashMap::new();
        map.insert("KEY1".to_string(), "value1".to_string());
        map.insert("KEY2".to_string(), "value2".to_string());

        let secret_map = SecretHashMap::from_hashmap(map);

        assert_eq!(secret_map.len(), 2);
        assert_eq!(secret_map.get("KEY1").unwrap().expose_secret(), "value1");
        assert_eq!(secret_map.get("KEY2").unwrap().expose_secret(), "value2");
    }

    #[test]
    fn test_secret_hashmap_debug_output() {
        let mut secret_map = SecretHashMap::new();
        secret_map.insert(
            "DATABASE_PASSWORD".to_string(),
            "super-secret-123".to_string(),
        );
        secret_map.insert("LOG_LEVEL".to_string(), "info".to_string());

        let debug_output = format!("{:?}", secret_map);

        // Should contain masked password
        assert!(debug_output.contains("***"));

        // Should NOT contain full password
        assert!(!debug_output.contains("super-secret-123"));

        // Both keys should be present
        assert!(debug_output.contains("DATABASE_PASSWORD"));
        assert!(debug_output.contains("LOG_LEVEL"));
    }

    #[test]
    fn test_secret_hashmap_expose_all() {
        let mut secret_map = SecretHashMap::new();
        secret_map.insert("KEY1".to_string(), "value1".to_string());
        secret_map.insert("KEY2".to_string(), "value2".to_string());

        let exposed = secret_map.expose_all();

        assert_eq!(exposed.get("KEY1").unwrap(), "value1");
        assert_eq!(exposed.get("KEY2").unwrap(), "value2");
    }

    #[test]
    fn test_secret_hashmap_to_masked() {
        let mut secret_map = SecretHashMap::new();
        secret_map.insert("API_KEY".to_string(), "secret-api-key-12345".to_string());
        secret_map.insert("LOG_LEVEL".to_string(), "info".to_string());

        let masked = secret_map.to_masked_hashmap();

        // Sensitive key should be masked
        let api_key = masked.get("API_KEY").unwrap();
        assert!(api_key.contains("***"));
        assert!(api_key.ends_with("345"));

        // Non-sensitive key should be unchanged
        assert_eq!(masked.get("LOG_LEVEL").unwrap(), "info");
    }

    #[test]
    fn test_secret_hashmap_serialization() {
        let mut secret_map = SecretHashMap::new();
        secret_map.insert("PASSWORD".to_string(), "secret123".to_string());
        secret_map.insert("USER".to_string(), "admin".to_string());

        let yaml = serde_norway::to_string(&secret_map).unwrap();

        // Should serialize full values
        assert!(yaml.contains("secret123"));
        assert!(yaml.contains("admin"));
    }

    #[test]
    fn test_secret_hashmap_deserialization() {
        let yaml = r#"
PASSWORD: secret123
USER: admin
"#;
        let secret_map: SecretHashMap = serde_norway::from_str(yaml).unwrap();

        assert_eq!(
            secret_map.get("PASSWORD").unwrap().expose_secret(),
            "secret123"
        );
        assert_eq!(secret_map.get("USER").unwrap().expose_secret(), "admin");
    }

    #[test]
    fn test_secret_hashmap_equality() {
        let mut map1 = SecretHashMap::new();
        map1.insert("KEY1".to_string(), "value1".to_string());

        let mut map2 = SecretHashMap::new();
        map2.insert("KEY1".to_string(), "value1".to_string());

        assert_eq!(map1, map2);
    }
}
