# Secret Migration Progress - Current Status

**Last Updated**: 2025-10-15
**Branch**: `feat/better-secrets-handling`
**Latest Commit**: `b713180` - "feat: migrate core secrets to MaskedSecret (Phase 1)"

## Executive Summary

We're migrating the scotty codebase from plain String secrets to the `secrecy` crate's memory-protected types. This provides:
- Memory zeroization on drop (prevents memory dump exposure)
- Partial masking in debug output (prevents accidental log leaks)
- Type-safe access requiring explicit `.expose_secret()` calls
- Easy security auditing via grep

## ✅ Completed Work

### Phase 0: Foundation (Committed: df22d7a)

**Created New Types**:
1. **`MaskedSecret`** (`scotty-core/src/utils/secret.rs`)
   - Wraps `SecretString` from secrecy crate
   - Custom `Debug` shows partial masking: `"*****-******-********-123"`
   - Custom `Display` shows partial masking
   - `Serialize` outputs full value (for docker-compose YAML)
   - `Deserialize` reads from YAML
   - Memory automatically zeroized on drop
   - **13 passing tests**

2. **`SecretHashMap`** (`scotty-core/src/utils/secret.rs`)
   - Wraps `HashMap<String, MaskedSecret>`
   - Smart `Debug` output shows all values masked
   - `to_masked_hashmap()` - for API responses (masks sensitive keys only)
   - `expose_all()` - for docker-compose/process execution (full values)
   - `Serialize`/`Deserialize` for YAML
   - **13 passing tests**

**Dependencies Added**:
```toml
secrecy = { version = "0.10", features = ["serde"] }
zeroize = "1.8"
```

**Documentation Created**:
- `docs/SECRET_HANDLING_ANALYSIS.md` - Analysis of current secret flow and gaps
- `docs/MASKEDSECRET_MIGRATION.md` - Complete migration plan
- `docs/CUSTOM_SECRET_TYPE.md` - Design rationale for MaskedSecret

### Phase 1: Core Secrets (Committed: b713180)

**Migrated Fields**:

1. **Docker Registry Password**
   - File: `scotty-core/src/settings/docker.rs`
   - Changed: `DockerRegistrySettings.password: String` → `MaskedSecret`
   - Updated: `scotty/src/docker/state_machine_handlers/run_docker_login_handler.rs:48`
   - Usage: `registry.password.expose_secret()`

2. **GitLab API Token**
   - File: `scotty-core/src/settings/notification_services.rs`
   - Changed: `GitlabSettings.token: String` → `MaskedSecret`
   - Updated: `scotty/src/notification/gitlab.rs:92`
   - Usage: `settings.token.expose_secret()`

3. **Mattermost Webhook Hook ID**
   - File: `scotty-core/src/settings/notification_services.rs`
   - Changed: `MattermostSettings.hook_id: String` → `MaskedSecret`
   - Updated: `scotty/src/notification/mattermost.rs:38-41`
   - Usage: `settings.hook_id.expose_secret()`

4. **OnePassword Secrets**
   - File: `scotty/src/onepassword/lookup.rs`
   - Changed return type: `lookup_password() -> anyhow::Result<String>` → `anyhow::Result<MaskedSecret>`
   - Updated caller in `resolve_environment_variables()` to use `.expose_secret()`

**Tests Updated**:
- File: `scotty/src/settings/config.rs`
- Updated assertions in `test_docker_registry_password_from_env()`
- Updated assertions in `test_notificaction_service_settings()`
- All tests use `.expose_secret()` for comparisons

**Test Results**:
- ✅ 41 scotty-core tests passing
- ✅ 31 scotty tests passing (5 ignored - unrelated)
- ✅ Total: 72 tests passing

**Files Changed in Phase 1**: 9 files
- `scotty-core/src/settings/docker.rs`
- `scotty-core/src/settings/notification_services.rs`
- `scotty/src/docker/state_machine_handlers/run_docker_login_handler.rs`
- `scotty/src/notification/gitlab.rs`
- `scotty/src/notification/mattermost.rs`
- `scotty/src/onepassword/lookup.rs`
- `scotty/src/settings/config.rs`
- `scotty-core/src/utils/secret.rs` (doctest fix)
- `docs/MASKEDSECRET_MIGRATION.md` (updated)

## 🚧 Next Steps: Phase 2 - Environment Variables

### Overview

Phase 2 will migrate environment variables from `HashMap<String, String>` to `SecretHashMap`. This is the largest phase, affecting ~50 locations in the codebase.

### Key Changes Needed

#### 2.1 Migrate AppSettings.environment

**Location**: `scotty-core/src/apps/app_data/settings.rs`

**Current**:
```rust
pub struct AppSettings {
    pub environment: HashMap<String, String>,  // ❌
    // ...
}
```

**Target**:
```rust
use scotty_core::utils::secret::SecretHashMap;

pub struct AppSettings {
    pub environment: SecretHashMap,  // ✅
    // ...
}
```

**Impact**: This will require updating ~50 call sites.

#### 2.2 Update resolve_environment_variables

**Location**: `scotty/src/onepassword/lookup.rs`

**Current**:
```rust
pub async fn resolve_environment_variables(
    settings: &Settings,
    env: &HashMap<String, String>,
) -> HashMap<String, String>
```

**Target**:
```rust
pub async fn resolve_environment_variables(
    settings: &Settings,
    env: &SecretHashMap,
) -> SecretHashMap
```

#### 2.3 Update LoadBalancer Implementations

**Files**:
- `scotty/src/docker/loadbalancer/traefik.rs`
- `scotty/src/docker/loadbalancer/haproxy.rs`

**Change Pattern**:
```rust
// BEFORE
for (key, value) in resolved_environment {
    environment.insert(key.clone(), value.clone());
}

// AFTER
for (key, value) in resolved_environment.iter() {
    environment.insert(key.clone(), value.expose_secret().to_string());
}
```

#### 2.4 Update docker-compose Config Generation

**Locations** (search for environment variable usage):
- Docker compose override generation
- Service environment setup
- Task execution environment

#### 2.5 Simplify SecureJson (Optional)

**Current**: `scotty/src/api/secure_response.rs` manually masks environment variables

**After**: `SecretHashMap` has built-in masking via `to_masked_hashmap()`

**Potential simplification**:
```rust
// BEFORE
impl IntoResponse for SecureJson<AppData> {
    fn into_response(self) -> axum::response::Response {
        let mut app_data = self.0;
        if let Some(settings) = app_data.settings.as_mut() {
            let masked_env = mask_sensitive_env_map(&settings.environment);
            settings.environment = masked_env;
        }
        Json(app_data).into_response()
    }
}

// AFTER
impl IntoResponse for SecureJson<AppData> {
    fn into_response(self) -> axum::response::Response {
        let mut app_data = self.0;
        if let Some(settings) = app_data.settings.as_mut() {
            let masked_env = settings.environment.to_masked_hashmap();
            settings.environment = SecretHashMap::from_hashmap(masked_env);
        }
        Json(app_data).into_response()
    }
}
```

### Search Strategy for Phase 2

To find all locations that need updating:

```bash
# Find AppSettings.environment usage
rg "\.environment" --type rust | grep -v test | grep -v docs

# Find HashMap<String, String> environment patterns
rg "HashMap<String, String>" scotty-core/src/apps/ scotty/src/

# Find resolve_environment_variables calls
rg "resolve_environment_variables" --type rust

# Find places where env vars are iterated
rg "for.*in.*environment" --type rust
```

### Estimated Impact

- **Files to modify**: 15-20 files
- **Lines to change**: 50-80 locations
- **Test updates**: 10-15 tests
- **Time estimate**: 2-3 days

## 📋 Remaining Phases

### Phase 3: Cleanup & Optimization (Week 4)

After Phase 2, we can:

1. **Consider Removing SecureJson**
   - Evaluate if `SecretHashMap`'s built-in masking makes it redundant
   - Keep initially, remove later if proven unnecessary

2. **Add Logging Sanitization Layer**
   - Belt-and-suspenders protection
   - Tracing subscriber that scans for exposed secrets
   - Logs warnings if secrets appear in logs

3. **Security Audit**
   - Grep for all `.expose_secret()` calls
   - Verify each usage is justified
   - Document why each exposure is necessary

4. **Performance Testing**
   - Benchmark to ensure < 1% overhead
   - Test with real workloads

5. **Documentation**
   - Update developer docs
   - Add usage guidelines for MaskedSecret
   - Team training on new patterns

## 🔑 Key Decisions Made

### Why MaskedSecret Instead of Raw SecretString?

**Decision**: Create `MaskedSecret` newtype wrapper instead of using `SecretString` directly.

**Rationale**:
- `SecretString`'s Debug shows `"[REDACTED String]"` - not helpful for debugging
- Our `MaskedSecret` shows partial value: `"****1234"` - useful for debugging
- Still get all memory protection benefits from underlying `SecretString`
- Best of both worlds: security + usability

### Why Include Notification Credentials in Phase 1?

**Decision**: Migrate GitLab tokens and Mattermost hook IDs in Phase 1.

**Rationale**:
- These are clear secrets (API tokens, webhook credentials)
- Small impact (2 fields, 2 handlers)
- Natural grouping with docker registry passwords
- Better to protect sooner than later

### Why Serialize Full Values in YAML?

**Decision**: `MaskedSecret::serialize()` outputs the full secret value.

**Rationale**:
- Docker containers need actual secrets to function
- `docker-compose.override.yml` must contain real values
- Masking is only for display (logs, API responses)
- File permissions protect YAML files on disk

## 🧪 Testing Strategy

### Phase 1 Tests (All Passing ✅)

- [x] DockerRegistrySettings with MaskedSecret
- [x] Docker login with `.expose_secret()`
- [x] OnePassword lookup returns MaskedSecret
- [x] GitlabSettings.token with MaskedSecret
- [x] MattermostSettings.hook_id with MaskedSecret
- [x] Notification handlers with `.expose_secret()`
- [x] Existing docker registry tests still pass
- [x] Existing notification service tests still pass

### Phase 2 Tests (TODO)

- [ ] AppSettings serialization/deserialization
- [ ] SecretHashMap in docker-compose override
- [ ] Environment variables in task execution
- [ ] Load balancer configs with SecretHashMap
- [ ] All existing API tests still pass
- [ ] Verify secrets NOT in docker-compose YAML (should be full values)
- [ ] Verify secrets ARE masked in API responses
- [ ] Verify secrets ARE masked in debug logs

### Phase 3 Tests (TODO)

- [ ] No secrets in log output (integration test)
- [ ] Memory zeroization (unit test with custom Drop)
- [ ] Security audit: search for `.expose_secret()` calls
- [ ] Performance: no significant overhead (< 1%)

## 🐛 Known Issues & Gotchas

### Issue 1: Deserialization from YAML

**Problem**: MaskedSecret needs to deserialize from plain strings in YAML config files.

**Solution**: Custom `Deserialize` impl that wraps incoming strings:
```rust
impl<'de> Deserialize<'de> for MaskedSecret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Ok(MaskedSecret::new(value))
    }
}
```

**Status**: ✅ Implemented and working

### Issue 2: Test Comparisons

**Problem**: Can't directly compare `MaskedSecret` with `&str` in tests.

**Wrong**:
```rust
assert_eq!(settings.password, "test_password");  // ❌ Type mismatch
```

**Correct**:
```rust
assert_eq!(settings.password.expose_secret(), "test_password");  // ✅
```

**Status**: ✅ All tests updated

### Issue 3: Masking Algorithm Preserves Structure

**Behavior**: The masking function preserves dashes and structure:
- Input: `"super-secret-password-123"`
- Output: `"*****-******-********-123"`

**Impact**: Makes debugging easier (can see word boundaries), but shows more structure than pure `"****123"`.

**Decision**: Keep current behavior - usability benefit outweighs minor structure leak.

**Status**: ✅ Working as designed

## 📝 Important Code Patterns

### Pattern 1: Creating MaskedSecret

```rust
// From String
let secret = MaskedSecret::new("my-secret".to_string());

// From &str
let secret = MaskedSecret::from_str("my-secret");
```

### Pattern 2: Using MaskedSecret

```rust
// Debug output (logs) - shows masked
println!("{:?}", secret);  // "***cret"

// Explicit access when needed
let password = secret.expose_secret();  // "my-secret"
docker_login(username, password);  // Pass to external command
```

### Pattern 3: Creating SecretHashMap

```rust
// New empty map
let mut env = SecretHashMap::new();
env.insert("KEY".to_string(), "value".to_string());

// From existing HashMap
let plain_map: HashMap<String, String> = ...;
let secret_map = SecretHashMap::from_hashmap(plain_map);
```

### Pattern 4: Using SecretHashMap

```rust
// For docker-compose YAML (full values)
let yaml = serde_yaml::to_string(&env)?;  // Full values serialized

// For API responses (masked)
let masked = env.to_masked_hashmap();  // Returns HashMap<String, String>
Json(masked)

// For process execution (full values)
let exposed = env.expose_all();  // Returns HashMap<String, String>
Command::new("docker").envs(exposed).spawn()?;
```

## 🔍 How to Resume Work

### 1. Checkout the branch

```bash
git checkout feat/better-secrets-handling
```

### 2. Verify current state

```bash
git log --oneline -3
# Should show:
# b713180 feat: migrate core secrets to MaskedSecret (Phase 1)
# df22d7a feat: implement MaskedSecret and SecretHashMap (Phase 0)
# ...

cargo test --lib
# Should show: 72 tests passing
```

### 3. Start Phase 2

```bash
# Search for environment variable usage
rg "\.environment" --type rust scotty-core/src/apps/ scotty/src/

# Find resolve_environment_variables calls
rg "resolve_environment_variables" --type rust

# Review migration plan
cat docs/MASKEDSECRET_MIGRATION.md
```

### 4. Begin with AppSettings

Start by modifying `scotty-core/src/apps/app_data/settings.rs`:

```rust
use crate::utils::secret::SecretHashMap;

pub struct AppSettings {
    pub environment: SecretHashMap,  // Change from HashMap<String, String>
    // ...
}
```

Then use compiler errors to guide remaining changes.

## 📚 Reference Documentation

- **Migration Plan**: `docs/MASKEDSECRET_MIGRATION.md`
- **Analysis**: `docs/SECRET_HANDLING_ANALYSIS.md`
- **Design Rationale**: `docs/CUSTOM_SECRET_TYPE.md`
- **This Progress Doc**: `docs/SECRET_MIGRATION_PROGRESS.md`

## 🎯 Success Criteria

- [x] Phase 0: Foundation types implemented with tests
- [x] Phase 1: Core secrets migrated (docker, onepassword, notifications)
- [ ] Phase 2: Environment variables migrated
- [ ] Phase 3: Cleanup and optimization
- [ ] All tests passing
- [ ] No secrets visible in debug logs
- [ ] YAML serialization unchanged (docker-compose still works)
- [ ] API responses show partial masking
- [ ] No performance degradation (< 1% overhead)
- [ ] Security audit complete (`.expose_secret()` calls justified)

## 🤝 Collaboration Notes

### For Code Review

**What to look for**:
1. Are all `.expose_secret()` calls justified?
2. Are there any places where secrets might still leak to logs?
3. Are the tests comprehensive enough?
4. Is the masking behavior consistent?

**What NOT to worry about**:
1. The overhead of MaskedSecret is negligible
2. YAML serialization is intentionally full values (for docker-compose)
3. Some tests use `.expose_secret()` - this is expected for assertions

### For Testing

**Manual test checklist**:
- [ ] Create app with sensitive environment variables
- [ ] Check docker-compose.override.yml has full values
- [ ] Check API response shows masked values
- [ ] Check logs show masked values
- [ ] Verify app containers receive correct environment variables

## 📊 Git History

```
feat/better-secrets-handling
  ├─ b713180 feat: migrate core secrets to MaskedSecret (Phase 1)
  │   └─ 9 files changed: docker registry, notifications, onepassword
  │
  └─ df22d7a feat: implement MaskedSecret and SecretHashMap (Phase 0)
      └─ Created foundation types with 26 passing tests
```

## 🔒 Security Notes

### What's Protected Now (Phase 1 Complete)

- ✅ Docker registry passwords - memory protected
- ✅ GitLab API tokens - memory protected
- ✅ Mattermost webhook credentials - memory protected
- ✅ OnePassword secrets - memory protected

### What's Still Exposed (Phase 2 TODO)

- ⚠️ Environment variables in AppSettings - plain HashMap
- ⚠️ Resolved environment variables - plain HashMap
- ⚠️ Task execution environment - plain HashMap

### What's Intentionally Exposed

- ✅ Secrets in docker-compose.override.yml files - needed for containers
- ✅ Secrets passed to external commands - needed for functionality
- ✅ Secrets in tests - needed for assertions

All intentional exposures use explicit `.expose_secret()` calls for audit trail.

---

**End of Progress Document**

Last commit: `b713180`
Next task: Begin Phase 2 by migrating `AppSettings.environment`
Status: Ready to continue ✅
