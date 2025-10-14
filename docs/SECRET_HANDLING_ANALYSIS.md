# Secret Handling Analysis - UPDATED

## Executive Summary

**CRITICAL INSIGHT**: Environment variables and OnePassword secrets are the PRIMARY secret entry points in Scotty. They currently flow through the system as plain `String` values, living in memory unprotected. This analysis re-evaluates the `secrecy` crate as a solution for proper secret lifecycle management, not just display masking.

**Key Finding**: The current approach is "mask on display" but secrets live as `HashMap<String, String>` throughout their lifecycle. We need "protect in memory" from load to use.

## Current Implementation

### Core Components

1. **`scotty-core/src/utils/sensitive_data.rs`**
   - Pattern-based detection of sensitive keys
   - Value masking with configurable visible suffix
   - URI credential masking
   - HashMap masking utility

2. **`scotty/src/api/secure_response.rs`**
   - `SecureJson` wrapper for API responses
   - Ensures secrets are masked only in API responses, not in storage
   - Implements `IntoResponse` for AppData, AppDataVec, AppSettings, RunningAppContext

### Sensitive Patterns

```rust
const SENSITIVE_PATTERNS: [&str; 10] = [
    "password", "secret", "token", "key", "auth",
    "credential", "cert", "private", "api_key", "pass",
];
```

### Masking Strategy

- **Short values (< 12 chars)**: Last 2 chars visible
- **Long values (>= 12 chars)**: Last 4 chars visible
- **Dashes preserved** in their original positions
- **URI credentials**: Only password part is masked

## Secret Lifecycle Analysis (CRITICAL)

### Current Secret Flow

```
┌─────────────────────────────────────────────────────────────┐
│ 1. SECRET ENTRY POINTS                                       │
├─────────────────────────────────────────────────────────────┤
│ • OnePassword API → lookup_password() → String              │
│ • User-provided env vars → HashMap<String, String>          │
│ • Config file (docker registry) → String                    │
└───────────────────┬─────────────────────────────────────────┘
                    │
                    ↓ Plain String in memory
┌─────────────────────────────────────────────────────────────┐
│ 2. STORAGE & PROCESSING (⚠️ UNPROTECTED)                     │
├─────────────────────────────────────────────────────────────┤
│ • AppSettings.environment: HashMap<String, String>          │
│ • resolve_environment_variables() → HashMap<String, String> │
│ • LoadBalancer configs → HashMap<String, String>            │
│ • Duration in memory: Minutes to hours                      │
│ • Protection: NONE - plain String                           │
└───────────────────┬─────────────────────────────────────────┘
                    │
                    ↓ Multiple paths
┌─────────────────────────────────────────────────────────────┐
│ 3. USAGE & EXPOSURE POINTS                                   │
├─────────────────────────────────────────────────────────────┤
│ • API Response → SecureJson masks → ✅ Protected            │
│ • YAML files → Unmasked → ✅ Intentional                    │
│ • Docker login → Command args → ⚠️ Process list exposure    │
│ • Task logs → stdout/stderr → ❌ UNPROTECTED                │
│ • Error messages → tracing::error! → ❌ UNPROTECTED         │
│ • Debug output → {:?} → ❌ UNPROTECTED                      │
└─────────────────────────────────────────────────────────────┘
```

### Critical Problems

1. **Memory Exposure**: Secrets as plain `String` can be:
   - Read from memory dumps
   - Captured by debuggers
   - Logged accidentally via Debug trait
   - Leaked in panic messages

2. **No Zeroization**: When secrets are dropped, they remain in memory until overwritten

3. **Accidental Logging**: Easy to log secrets via:
   ```rust
   error!("Config error: {:?}", settings); // Logs ALL env vars!
   debug!("Resolved: {:?}", resolved_env); // Exposes secrets!
   ```

4. **No Type Safety**: Nothing prevents:
   ```rust
   let password = registry.password;  // Easy to copy
   println!("{}", password);          // Easy to leak
   ```

### Secret Usage Patterns in Scotty

| Location | Count | Current Type | Secret? | Risk |
|----------|-------|--------------|---------|------|
| `AppSettings.environment` | Core | `HashMap<String, String>` | Mixed | HIGH |
| `resolve_environment_variables()` | 1 | Returns `HashMap<String, String>` | Yes | HIGH |
| `OnePassword::lookup_password()` | 1 | Returns `String` | Yes | HIGH |
| `DockerRegistrySettings.password` | 1 | `String` | Yes | HIGH |
| Load balancer configs | ~20 | `HashMap<String, String>` | Mixed | MEDIUM |
| Task execution env vars | Multiple | `HashMap<String, String>` | Mixed | HIGH |

**Total exposure**: Secrets flow through 50+ locations as plain String

## Test Coverage Inventory

### Existing Tests

#### 1. Core Masking Tests (`scotty-core/src/utils/sensitive_data.rs`)

| Test | Coverage | Location |
|------|----------|----------|
| `test_is_sensitive` | ✅ Sensitive key detection | Line 141 |
| `test_mask_sensitive_value` | ✅ Value masking logic | Line 164 |
| `test_uri_with_credentials_detection` | ✅ URI credential detection | Line 190 |
| `test_mask_uri_credentials` | ✅ URI password masking | Line 216 |
| `test_mask_sensitive_env_map` | ✅ HashMap masking | Line 244 |

#### 2. API Response Tests (`scotty/src/api/secure_response_test.rs`)

| Test | Coverage | Location |
|------|----------|----------|
| `test_secure_json_masks_app_settings_env_vars` | ✅ AppSettings masking in API | Line 62 |
| `test_secure_json_masks_app_data_env_vars` | ✅ AppData masking in API | Line 118 |
| `test_secure_json_masks_app_data_vec_env_vars` | ✅ AppDataVec masking in API | Line 168 |
| `test_secure_json_masks_running_app_context_env_vars` | ✅ RunningAppContext masking | Line 227 |

#### 3. Storage Tests (`scotty-core/src/apps/app_data/settings.rs`)

| Test | Coverage | Location |
|------|----------|----------|
| `test_environment_vars_not_masked_in_yaml_file` | ✅ Ensures secrets are NOT masked in files | Line 179 |

#### 4. Docker Compose Tests (`scotty/src/docker/state_machine_handlers/create_load_balancer_config.rs`)

| Test | Coverage | Location |
|------|----------|----------|
| `test_docker_compose_override_contains_unmasked_secrets` | ✅ Ensures secrets are NOT masked in docker-compose.override.yml | Line 107 |

## Test Coverage Gaps

### Critical Gaps (Security Risks)

1. **Task Output Logging** ❌
   - **Location**: `scotty/src/tasks/manager.rs:205-212`
   - **Risk**: Command output (stdout/stderr) captured without masking
   - **Example**: `docker login -p password` errors might expose passwords
   - **Test Needed**: Verify sensitive data is masked in task logs

2. **Docker Registry Password Usage** ❌
   - **Location**: `scotty/src/docker/state_machine_handlers/run_docker_login_handler.rs:42-49`
   - **Risk**: Password passed as command argument, might appear in process lists
   - **Test Needed**: Verify password isn't logged or exposed in task details

3. **OnePassword Lookup Errors** ⚠️
   - **Location**: `scotty/src/onepassword/lookup.rs:22-24`
   - **Risk**: Error messages might expose partial secret values
   - **Test Needed**: Verify error messages don't leak secrets

### Medium Priority Gaps

4. **Environment Variable Substitution** ⚠️
   - **Location**: `scotty/src/onepassword/env_substitution.rs`
   - **Current Tests**: Basic substitution tested
   - **Gap**: No tests for error cases that might expose secrets

5. **Config File Parsing** ⚠️
   - **Location**: `scotty/src/settings/config.rs`
   - **Gap**: No tests verifying registry passwords are not logged during config loading

6. **GitLab Notification Tokens** ⚠️
   - **Location**: `scotty/src/notification/gitlab.rs`
   - **Gap**: No tests for token handling in notifications

### Low Priority Gaps

7. **Basic Auth Credentials** ℹ️
   - Currently stored as plain `Option<(String, String)>`
   - Used in load balancer configs (Traefik, HAProxy)
   - Tests exist for functionality but not for secret handling

8. **API Server JWT/Bearer Tokens** ℹ️
   - **Location**: `scotty-core/src/settings/api_server.rs`
   - No specific tests for token masking in logs/errors

## Secrecy Crate Analysis

### Key Features

1. **`Secret<T>` / `SecretBox<T>`**: Wrapper type for secrets
2. **`ExposeSecret` trait**: Explicit, auditable secret access
3. **`Zeroize` integration**: Memory wiped on drop
4. **Debug protection**: Secrets show as `"[REDACTED]"` in debug output
5. **Serde support**: Optional, requires explicit opt-in for serialization

### Comparison with Current Solution

| Feature | Current Solution | Secrecy Crate | Winner |
|---------|-----------------|---------------|---------|
| **Type Safety** | None (plain String) | Strong (Secret<String>) | 🏆 Secrecy |
| **Memory Security** | No zeroization | Automatic zeroize on drop | 🏆 Secrecy |
| **Debug Output** | Manual masking | Automatic `[REDACTED]` | 🏆 Secrecy |
| **Audit Trail** | Implicit access | Explicit `.expose_secret()` | 🏆 Secrecy |
| **API Response Masking** | Custom SecureJson | Need custom impl | 🏆 Current |
| **Serde Integration** | Works by default | Requires feature flags | 🏆 Current |
| **URI Credential Masking** | Built-in | Need custom impl | 🏆 Current |
| **Partial Masking** | Configurable (last N chars) | All-or-nothing | 🏆 Current |
| **Learning Curve** | Low | Medium | 🏆 Current |

### Pros of Secrecy Crate

✅ **Type Safety**: Compile-time enforcement that secrets are handled explicitly
✅ **Memory Security**: Automatic zeroization prevents secrets lingering in memory
✅ **Debug Protection**: Prevents accidental logging via Debug trait
✅ **Audit Trail**: `.expose_secret()` calls are easily grep-able for security audits
✅ **Industry Standard**: Well-tested, widely used in Rust ecosystem
✅ **Zero-copy**: Minimal performance overhead

### Cons of Secrecy Crate

❌ **All-or-Nothing**: Cannot show partial values (last 4 chars) for debugging
❌ **Boilerplate**: Need to wrap/unwrap secrets throughout codebase
❌ **Migration Cost**: Significant refactoring required
❌ **Serde Complexity**: Requires careful feature flag management
❌ **API Response Masking**: Would still need custom logic for SecureJson behavior

## Re-Evaluated Recommendations

### Why Secrecy Crate is Now STRONGLY RECOMMENDED

Given that environment variables and OnePassword secrets are the primary secret sources:

1. **Memory Protection**: Secrets should be protected from the moment they're loaded until used
2. **Type Safety**: Prevent accidental copying and logging
3. **Audit Trail**: Make secret access explicit and searchable
4. **Defense in Depth**: Multiple layers of protection beyond display masking

### Proposed SecretString Type

```rust
use secrecy::{Secret, ExposeSecret};
use std::collections::HashMap;

// Type alias for clarity
pub type SecretString = Secret<String>;

// For environment variables with mixed sensitivity
pub type SecretHashMap = HashMap<String, SecretString>;
```

### Migration Strategy

#### Phase 1: Core Secret Types (Week 1)

```rust
// BEFORE
pub struct DockerRegistrySettings {
    pub registry: String,
    pub username: String,
    pub password: String,  // ❌ Exposed
}

// AFTER
pub struct DockerRegistrySettings {
    pub registry: String,
    pub username: String,
    pub password: SecretString,  // ✅ Protected
}

// BEFORE
async fn lookup_password(settings: &Settings, op_uri: &str) -> anyhow::Result<String>

// AFTER
async fn lookup_password(settings: &Settings, op_uri: &str) -> anyhow::Result<SecretString>
```

#### Phase 2: Environment Variables (Week 2-3)

Challenge: Environment variables contain both secrets AND non-secrets.

**Solution: Smart SecretHashMap**

```rust
use secrecy::{Secret, ExposeSecret};

#[derive(Debug, Clone)]
pub struct SecretHashMap {
    map: HashMap<String, SecretString>,
}

impl SecretHashMap {
    // Create from plain HashMap, wrapping all values
    pub fn from_hashmap(map: HashMap<String, String>) -> Self {
        Self {
            map: map.into_iter()
                .map(|(k, v)| (k, Secret::new(v)))
                .collect()
        }
    }

    // Get a reference to secret - requires explicit expose
    pub fn get(&self, key: &str) -> Option<&SecretString> {
        self.map.get(key)
    }

    // Convert to plain HashMap - only when truly needed
    pub fn expose_all(&self) -> HashMap<String, String> {
        self.map.iter()
            .map(|(k, v)| (k.clone(), v.expose_secret().clone()))
            .collect()
    }

    // For serialization to YAML (docker-compose.override.yml)
    pub fn to_yaml_map(&self) -> HashMap<String, String> {
        self.expose_all() // Intentionally expose for docker
    }
}

// Custom Debug that shows [REDACTED] for sensitive keys
impl std::fmt::Debug for SecretHashMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut map = f.debug_map();
        for (k, v) in &self.map {
            if is_sensitive(k) {
                map.entry(k, &"[REDACTED]");
            } else {
                map.entry(k, &format!("[SECRET:{}]", v.expose_secret().len()));
            }
        }
        map.finish()
    }
}

// Serde support
impl Serialize for SecretHashMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as plain HashMap for YAML files
        self.expose_all().serialize(serializer)
    }
}
```

**Updated Types**:

```rust
// BEFORE
pub struct AppSettings {
    pub environment: HashMap<String, String>,  // ❌
    // ...
}

// AFTER
pub struct AppSettings {
    pub environment: SecretHashMap,  // ✅
    // ...
}

// BEFORE
pub async fn resolve_environment_variables(
    settings: &Settings,
    env: &HashMap<String, String>,
) -> HashMap<String, String>

// AFTER
pub async fn resolve_environment_variables(
    settings: &Settings,
    env: &SecretHashMap,
) -> SecretHashMap
```

#### Phase 3: Usage Points (Week 3-4)

Only expose secrets where absolutely necessary:

```rust
// Docker login - MUST expose
async fn docker_login(&self, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
    let registry = get_registry_settings(&context)?;

    // Explicit exposure only where needed
    let args = vec![
        "login",
        &registry.registry,
        "-u",
        &registry.username,
        "-p",
        registry.password.expose_secret(),  // ✅ Explicit
    ];

    // ...
}

// Docker compose env vars - MUST expose
async fn write_compose_override(&self, env: &SecretHashMap) -> anyhow::Result<()> {
    let yaml_env = env.to_yaml_map();  // ✅ Explicit conversion
    // Write to file...
}

// Process execution - MUST expose
pub async fn start_process(
    &self,
    env: &SecretHashMap,
    // ...
) -> Uuid {
    let plain_env = env.expose_all();  // ✅ Only at exec boundary

    Command::new(cmd)
        .envs(&plain_env)
        .spawn()?;
}

// API Response - Keep current masking for display
impl IntoResponse for SecureJson<AppData> {
    fn into_response(self) -> axum::response::Response {
        let mut app_data = self.0;

        if let Some(settings) = app_data.settings.as_mut() {
            // Convert SecretHashMap to masked plain HashMap
            let masked = mask_secret_hashmap(&settings.environment);
            // Serialize the masked version
        }

        Json(app_data).into_response()
    }
}
```

### Benefits of This Approach

1. **Memory Protection**: ✅ Secrets zeroized on drop
2. **Debug Safety**: ✅ `{:?}` prints `[REDACTED]` automatically
3. **Type Safety**: ✅ Cannot accidentally log/copy without `.expose_secret()`
4. **Audit Trail**: ✅ Search for `.expose_secret()` to find all usage
5. **Backward Compatible**: ✅ Serialization still works for YAML files
6. **API Masking**: ✅ Keep existing partial masking for debugging

### Short-Term (Immediate Actions - Days 1-2)

1. **Add Critical Missing Tests**
   ```rust
   // Priority 1: Task output masking
   #[tokio::test]
   async fn test_docker_login_password_not_in_task_output()

   // Priority 2: Registry password in command args
   #[tokio::test]
   async fn test_docker_login_command_args_masked()
   ```

2. **Enhance Logging**
   - Add logging sanitization middleware
   - Mask sensitive patterns in all log output
   - Test: Verify logs don't contain secrets

3. **Document Secret Flow**
   - Map all paths where secrets flow through the system
   - Document which components mask vs. preserve secrets

### Medium-Term (Consider for Next Major Version)

1. **Hybrid Approach**
   ```rust
   // Use secrecy crate for storage/transport
   pub struct DockerRegistrySettings {
       pub registry: String,
       pub username: String,
       pub password: Secret<String>,  // Changed!
   }

   // Keep current masking for API responses
   impl IntoResponse for SecureJson<AppData> {
       // ... existing logic with partial masking
   }
   ```

2. **Benefits of Hybrid**:
   - Type safety for internal handling
   - Memory security via zeroization
   - Flexible masking for external display
   - Gradual migration path

### Long-Term (Future Consideration)

1. **Full Secrecy Migration** (if benefits justify cost):
   - Estimate: 2-3 weeks of development
   - Risk: Breaking changes to API serialization
   - Benefit: Industry-standard secret handling

2. **Alternative: Enhanced Current Solution**:
   - Add memory zeroization to current masking
   - Implement wrapper type for type safety
   - Keep existing masking flexibility

## Decision Matrix

| Scenario | Recommendation |
|----------|---------------|
| **High security requirements** | Adopt secrecy crate (full or hybrid) |
| **Limited dev resources** | Enhance current solution + add tests |
| **Frequent secret debugging** | Keep current solution (partial masking useful) |
| **Compliance requirements** | Secrecy crate provides better audit trail |

## Action Plan

### Phase 1: Testing & Documentation (1-2 days)
- [ ] Add tests for task output masking
- [ ] Add tests for docker login password handling
- [ ] Document secret flow diagram
- [ ] Run security audit on existing code

### Phase 2: Quick Wins (2-3 days)
- [ ] Add logging sanitization
- [ ] Implement command argument masking
- [ ] Add error message sanitization

### Phase 3: Evaluation (1 week)
- [ ] Prototype hybrid approach
- [ ] Benchmark performance impact
- [ ] Assess migration effort
- [ ] Get team feedback

### Phase 4: Decision & Implementation (2-4 weeks)
- [ ] Choose approach based on evaluation
- [ ] Implement chosen solution
- [ ] Update all tests
- [ ] Security review

## Conclusion - UPDATED RECOMMENDATION

**Current State**: The existing solution masks secrets for API responses, but secrets live as plain `String` in memory throughout their lifecycle. This exposes them to:
- Memory dumps
- Debug logging
- Error messages
- Accidental copying

**STRONG RECOMMENDATION: Adopt Secrecy Crate**

Given that **environment variables and OnePassword secrets are the primary secret sources**, we should:

### 1. **Immediate (Week 1)**: Implement Foundation
- Add `secrecy` crate dependency
- Create `SecretHashMap` wrapper type
- Implement Debug, Serialize traits
- Add comprehensive tests

### 2. **Phase 1 (Week 2)**: Core Secrets
- Migrate `DockerRegistrySettings.password` to `SecretString`
- Migrate `OnePassword::lookup_password()` to return `SecretString`
- Update docker login handler to use `.expose_secret()`
- Verify tests pass

### 3. **Phase 2 (Week 3-4)**: Environment Variables
- Migrate `AppSettings.environment` to `SecretHashMap`
- Update `resolve_environment_variables()` signature
- Update all LoadBalancer implementations
- Update docker-compose config generation

### 4. **Phase 3 (Week 5)**: API & Display
- Update `SecureJson` to work with `SecretHashMap`
- Maintain partial masking for API responses (last 4 chars)
- Add logging sanitization middleware
- Security audit

### Key Benefits

| Benefit | Impact | Timeline |
|---------|--------|----------|
| Memory zeroization | HIGH - Prevents memory dumps | Phase 1 |
| Debug protection | HIGH - Prevents accidental logs | Phase 1 |
| Type safety | MEDIUM - Compile-time enforcement | Phase 2 |
| Audit trail | MEDIUM - Easy to find exposure | Phase 2 |
| No behavioral change | LOW - Same functionality | All phases |

### Migration Effort

- **Estimated Time**: 4-5 weeks
- **Risk Level**: LOW (backward compatible)
- **Breaking Changes**: None (serialization unchanged)
- **Test Coverage**: Existing tests + new secrecy tests

### Why This is Better Than Current Solution

**Before**:
```rust
let password = settings.password;  // Plain String
println!("{:?}", settings);         // Logs password!
error!("Error: {:?}", env);         // Logs all secrets!
```

**After**:
```rust
let password = settings.password;          // Secret<String>
println!("{:?}", settings);                 // Shows "[REDACTED]"
error!("Error: {:?}", env);                 // Shows "SecretHashMap(..)"
let plain = password.expose_secret();       // ✅ Explicit & auditable
```

### Decision: YES to Secrecy Crate

The benefits of memory protection, debug safety, and type enforcement far outweigh the migration cost, especially given that secrets are the **primary concern** of this application.

**Start with Phase 1 immediately.**
