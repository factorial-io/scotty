# Custom Secret Type with Partial Masking

## The Question

Can we override the Debug output of `Secret<String>` to show partial masking (like `"****1234"`) instead of just `"[REDACTED]"`?

## Answer: YES - Multiple Approaches

### Option 1: Newtype Wrapper (RECOMMENDED)

Create a wrapper around `Secret<String>` with custom Debug:

```rust
use secrecy::{Secret, ExposeSecret, Zeroize};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use scotty_core::utils::sensitive_data::mask_sensitive_value;

/// A secret string with partial masking in Debug output
#[derive(Clone)]
pub struct MaskedSecret(Secret<String>);

impl MaskedSecret {
    pub fn new(value: String) -> Self {
        Self(Secret::new(value))
    }

    pub fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }
}

// Custom Debug with partial masking
impl std::fmt::Debug for MaskedSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Expose ONLY for masking, then immediately mask
        let masked = mask_sensitive_value(self.0.expose_secret());
        write!(f, "{}", masked)
    }
}

// Serialization support (for YAML files)
impl Serialize for MaskedSecret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize the full value (needed for docker-compose)
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

// Display shows masked value (useful for logging)
impl std::fmt::Display for MaskedSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let masked = mask_sensitive_value(self.0.expose_secret());
        write!(f, "{}", masked)
    }
}

// Still gets memory zeroization from Secret<T>
impl Drop for MaskedSecret {
    fn drop(&mut self) {
        // Secret<T> handles zeroization automatically
    }
}
```

**Benefits**:
- ✅ Memory protection from `secrecy` crate
- ✅ Partial masking in Debug/Display
- ✅ Full serialization for YAML
- ✅ Minimal code

**Usage**:
```rust
let secret = MaskedSecret::new("super-secret-password-123".to_string());

// Debug output
println!("{:?}", secret);  // Prints: ***************123

// Logging
tracing::info!("Password: {}", secret);  // Logs: Password: ***************123

// Explicit access
let plain = secret.expose_secret();  // "super-secret-password-123"

// Serialization (docker-compose)
let yaml = serde_yaml::to_string(&secret)?;  // "super-secret-password-123"
```

### Option 2: Custom SecretHashMap with Smart Debug

```rust
use std::collections::HashMap;

pub struct SecretHashMap {
    map: HashMap<String, MaskedSecret>,
}

impl std::fmt::Debug for SecretHashMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_map = f.debug_map();
        for (key, value) in &self.map {
            if is_sensitive(key) {
                // Sensitive keys: show masked value
                debug_map.entry(key, value);  // Uses MaskedSecret's Debug
            } else {
                // Non-sensitive: show length only
                debug_map.entry(key, &format!("[{}]", value.expose_secret().len()));
            }
        }
        debug_map.finish()
    }
}
```

**Debug Output**:
```rust
let mut env = SecretHashMap::new();
env.insert("DATABASE_PASSWORD", MaskedSecret::new("super-secret-123"));
env.insert("LOG_LEVEL", MaskedSecret::new("info"));

println!("{:?}", env);
// Output:
// SecretHashMap {
//     "DATABASE_PASSWORD": ***********123,
//     "LOG_LEVEL": [4]
// }
```

### Option 3: Full Custom Implementation (Most Control)

Build our own `Secret` type from scratch with exactly the features we need:

```rust
use zeroize::Zeroize;

pub struct OurSecret {
    value: String,
}

impl OurSecret {
    pub fn new(value: String) -> Self {
        Self { value }
    }

    pub fn expose_secret(&self) -> &str {
        &self.value
    }
}

impl std::fmt::Debug for OurSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let masked = mask_sensitive_value(&self.value);
        write!(f, "{}", masked)
    }
}

impl Drop for OurSecret {
    fn drop(&mut self) {
        // Zeroize memory
        self.value.zeroize();
    }
}

impl Clone for OurSecret {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone()
        }
    }
}

impl Serialize for OurSecret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.value.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OurSecret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(OurSecret::new(value))
    }
}
```

**Benefits**:
- ✅ Complete control over behavior
- ✅ No external dependency on secrecy crate
- ✅ Exactly what we need

**Drawbacks**:
- ❌ More code to maintain
- ❌ Need to ensure zeroization works correctly
- ❌ Reinventing the wheel

## Comparison

| Approach | Memory Safety | Partial Masking | Serialization | Complexity | Recommended |
|----------|---------------|-----------------|---------------|------------|-------------|
| Option 1: Newtype | ✅ From secrecy | ✅ Custom Debug | ✅ Custom impl | LOW | **⭐ YES** |
| Option 2: Smart HashMap | ✅ From secrecy | ✅ Per-key logic | ✅ Custom impl | MEDIUM | ⭐ YES |
| Option 3: Full custom | ⚠️ Manual zeroize | ✅ Full control | ✅ Full control | HIGH | ❌ No |

## Recommendation: Option 1 (Newtype Wrapper)

**Why**:
1. ✅ Leverages battle-tested `secrecy` crate for memory safety
2. ✅ Adds our partial masking on top
3. ✅ Minimal code (~50 lines)
4. ✅ Easy to test
5. ✅ Best of both worlds

**Implementation Path**:

```rust
// scotty-core/src/utils/secret.rs

use secrecy::{Secret, ExposeSecret};
use crate::utils::sensitive_data::mask_sensitive_value;

/// A secret string that shows partial masking in Debug output
/// while maintaining memory protection from the secrecy crate
#[derive(Clone)]
pub struct MaskedSecret(Secret<String>);

impl MaskedSecret {
    pub fn new(value: String) -> Self {
        Self(Secret::new(value))
    }

    pub fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }

    // Convert from plain string
    pub fn from_str(s: &str) -> Self {
        Self::new(s.to_string())
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

impl serde::Serialize for MaskedSecret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.expose_secret().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for MaskedSecret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(MaskedSecret::new(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_shows_masked() {
        let secret = MaskedSecret::new("super-secret-password-123".to_string());
        let debug = format!("{:?}", secret);

        // Should show masked value
        assert!(debug.contains("***"));
        assert!(debug.ends_with("123\""));

        // Should NOT show full value
        assert!(!debug.contains("super-secret-password"));
    }

    #[test]
    fn test_display_shows_masked() {
        let secret = MaskedSecret::new("api-key-12345".to_string());
        let display = format!("{}", secret);

        assert!(display.contains("***"));
        assert!(display.ends_with("45"));
    }

    #[test]
    fn test_expose_gives_full_value() {
        let secret = MaskedSecret::new("my-secret".to_string());
        assert_eq!(secret.expose_secret(), "my-secret");
    }

    #[test]
    fn test_serialization_preserves_value() {
        let secret = MaskedSecret::new("password123".to_string());
        let yaml = serde_yaml::to_string(&secret).unwrap();

        // Should serialize full value for docker-compose
        assert_eq!(yaml.trim(), "password123");
    }

    #[test]
    fn test_deserialization_works() {
        let yaml = "my-password";
        let secret: MaskedSecret = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(secret.expose_secret(), "my-password");
    }
}
```

## Impact on SecureJson

With `MaskedSecret`, **SecureJson becomes simpler**:

```rust
impl IntoResponse for SecureJson<AppData> {
    fn into_response(self) -> axum::response::Response {
        // MaskedSecret already handles Debug masking!
        // We only need to handle the serialization

        let app_data = self.0;
        // Serialization will show full values (for tests/debugging)
        // But logs will show masked values automatically

        Json(app_data).into_response()
    }
}
```

**OR** we still use SecureJson for API responses but:
- Logs show `"********123"` automatically (via Debug)
- API can choose between full masking or partial masking
- No more accidental leaks in logs

## Conclusion

**YES, we can override Debug output!**

**Recommendation**: Create `MaskedSecret` as a newtype wrapper around `Secret<String>`:
- Uses `secrecy` for memory protection
- Adds our partial masking for Debug/Display
- Keeps full serialization for YAML
- **Best of both worlds**: Security + Usability

This means:
1. ✅ We still need some form of masking logic (reuse existing)
2. ✅ We get memory protection from secrecy
3. ✅ We get useful debug output with partial masking
4. ✅ SecureJson can be simplified OR kept for API-specific formatting
