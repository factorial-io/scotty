# MaskedSecret Migration Plan

## ✅ Phase 0: Foundation (COMPLETED)

### What We Built

1. **`MaskedSecret` type** (`scotty-core/src/utils/secret.rs`)
   - Wraps `SecretString` from the secrecy crate
   - Custom `Debug` shows partial masking: `"***************123"`
   - Custom `Display` shows partial masking: `***************123`
   - `Serialize` outputs full value (for docker-compose YAML)
   - `Deserialize` reads from YAML
   - Memory zeroized on drop (from secrecy crate)
   - **13 tests - all passing ✅**

2. **`SecretHashMap` type** (`scotty-core/src/utils/secret.rs`)
   - Wraps `HashMap<String, MaskedSecret>`
   - Smart `Debug` output shows all values masked
   - `to_masked_hashmap()` - for API responses (masks sensitive keys only)
   - `expose_all()` - for docker-compose/process execution (full values)
   - `Serialize` outputs full values (for YAML)
   - `Deserialize` reads from YAML
   - **13 tests - all passing ✅**

### Current Behavior

```rust
// Debug output (logs)
let mut env = SecretHashMap::new();
env.insert("DB_PASSWORD".to_string(), "super-secret-123".to_string());
env.insert("LOG_LEVEL".to_string(), "info".to_string());

println!("{:?}", env);
// Output: { "DB_PASSWORD": "***********123", "LOG_LEVEL": "****" }

// Serialization (YAML files - docker-compose)
let yaml = serde_yaml::to_string(&env)?;
// Output: DB_PASSWORD: super-secret-123
//         LOG_LEVEL: info

// API Response masking
let masked = env.to_masked_hashmap();
// masked = { "DB_PASSWORD": "***********123", "LOG_LEVEL": "info" }
```

## Phase 1: Core Secrets (Next - 1 week)

### 1.1 Migrate DockerRegistrySettings

**Current**:
```rust
pub struct DockerRegistrySettings {
    pub registry: String,
    pub username: String,
    pub password: String,  // ❌ Plain String
}
```

**After**:
```rust
use scotty_core::utils::secret::MaskedSecret;

pub struct DockerRegistrySettings {
    pub registry: String,
    pub username: String,
    pub password: MaskedSecret,  // ✅ Protected
}
```

**Impact**:
- File: `scotty-core/src/settings/docker.rs`
- Usage: `scotty/src/docker/state_machine_handlers/run_docker_login_handler.rs:48`
- Change: `registry.password` → `registry.password.expose_secret()`

### 1.2 Migrate OnePassword lookup

**Current**:
```rust
async fn lookup_password(settings: &Settings, op_uri: &str) -> anyhow::Result<String>
```

**After**:
```rust
async fn lookup_password(settings: &Settings, op_uri: &str) -> anyhow::Result<MaskedSecret>
```

**Impact**:
- File: `scotty/src/onepassword/lookup.rs`
- Returns `MaskedSecret` instead of `String`
- Callers must use `.expose_secret()` explicitly

### 1.3 Migrate Notification Service Credentials

**Current**:
```rust
pub struct GitlabSettings {
    pub host: String,
    pub token: String,  // ❌ Plain String
}

pub struct MattermostSettings {
    pub host: String,
    pub hook_id: String,  // ❌ Plain String
}
```

**After**:
```rust
use crate::utils::secret::MaskedSecret;

pub struct GitlabSettings {
    pub host: String,
    pub token: MaskedSecret,  // ✅ Protected
}

pub struct MattermostSettings {
    pub host: String,
    pub hook_id: MaskedSecret,  // ✅ Protected
}
```

**Impact**:
- File: `scotty-core/src/settings/notification_services.rs`
- Usage: `scotty/src/notification/gitlab.rs:92`
- Usage: `scotty/src/notification/mattermost.rs:38`
- Changes: `settings.token` → `settings.token.expose_secret()`
- Changes: `settings.hook_id` → `settings.hook_id.expose_secret()`

## Phase 2: Environment Variables (2-3 weeks)

### 2.1 Migrate AppSettings.environment

**Current**:
```rust
pub struct AppSettings {
    pub environment: HashMap<String, String>,  // ❌
    // ...
}
```

**After**:
```rust
use scotty_core::utils::secret::SecretHashMap;

pub struct AppSettings {
    pub environment: SecretHashMap,  // ✅
    // ...
}
```

**Impact**: ~50 locations need updating

### 2.2 Update resolve_environment_variables

**Current**:
```rust
pub async fn resolve_environment_variables(
    settings: &Settings,
    env: &HashMap<String, String>,
) -> HashMap<String, String>
```

**After**:
```rust
pub async fn resolve_environment_variables(
    settings: &Settings,
    env: &SecretHashMap,
) -> SecretHashMap
```

### 2.3 Update LoadBalancer Implementations

**Files**:
- `scotty/src/docker/loadbalancer/traefik.rs`
- `scotty/src/docker/loadbalancer/haproxy.rs`

**Change**:
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

### 2.4 Simplify SecureJson

**Current** (`scotty/src/api/secure_response.rs`):
```rust
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
```

**After**:
```rust
impl IntoResponse for SecureJson<AppData> {
    fn into_response(self) -> axum::response::Response {
        let mut app_data = self.0;

        if let Some(settings) = app_data.settings.as_mut() {
            // SecretHashMap has built-in masking!
            let masked_env = settings.environment.to_masked_hashmap();
            settings.environment = SecretHashMap::from_hashmap(masked_env);
        }

        Json(app_data).into_response()
    }
}
```

**OR** (even simpler - if we customize serialization):
```rust
// SecretHashMap could have a custom "masked serialization" mode
// Then SecureJson becomes unnecessary for most cases
Json(app_data).into_response()  // Just works!
```

## Phase 3: Cleanup & Optimization (1 week)

### 3.1 Consider Removing SecureJson

Once `SecretHashMap` is fully integrated:

**Option A**: Keep SecureJson for API-specific formatting
- Pro: Explicit control over what gets masked
- Con: Extra boilerplate

**Option B**: Remove SecureJson, rely on MaskedSecret's Debug
- Pro: Simpler code, one less concept
- Con: Need to handle serialization modes

**Recommendation**: Start with Option A, evaluate Option B after full migration

### 3.2 Add Logging Sanitization

Even with MaskedSecret, add belt-and-suspenders logging protection:

```rust
// tracing subscriber layer that sanitizes secrets
pub struct SecretSanitizerLayer;

impl<S> Layer<S> for SecretSanitizerLayer {
    fn on_event(&self, event: &Event, ctx: Context<S>) {
        // Scan event for exposed secrets
        // Log warning if found
    }
}
```

## Benefits Summary

| Feature | Before | After | Benefit |
|---------|--------|-------|---------|
| **Memory Protection** | ❌ None | ✅ Zeroized on drop | Prevents memory dumps |
| **Debug Logging** | ❌ Full secrets | ✅ Partial masking | Prevents accidental leaks |
| **Type Safety** | ❌ Easy to copy | ✅ Must `.expose_secret()` | Compile-time enforcement |
| **Audit Trail** | ❌ No visibility | ✅ grep `.expose_secret()` | Easy security audits |
| **API Masking** | ✅ SecureJson | ✅ SecretHashMap | Simpler, built-in |
| **YAML Files** | ✅ Full values | ✅ Full values | No behavior change |

## Testing Strategy

### Phase 1 Tests
- [x] DockerRegistrySettings with MaskedSecret
- [x] Docker login with `.expose_secret()`
- [x] OnePassword lookup returns MaskedSecret
- [x] GitlabSettings.token with MaskedSecret
- [x] MattermostSettings.hook_id with MaskedSecret
- [x] Notification handlers with `.expose_secret()`
- [x] Existing docker registry tests still pass
- [x] Existing notification service tests still pass

### Phase 2 Tests
- [ ] AppSettings serialization/deserialization
- [ ] SecretHashMap in docker-compose override
- [ ] Environment variables in task execution
- [ ] Load balancer configs with SecretHashMap
- [ ] All existing API tests still pass

### Phase 3 Tests
- [ ] No secrets in log output (integration test)
- [ ] Memory zeroization (unit test with custom Drop)
- [ ] Security audit: search for `.expose_secret()` calls
- [ ] Performance: no significant overhead

## Migration Checklist

### Preparation
- [x] Add secrecy crate dependency
- [x] Implement MaskedSecret type
- [x] Implement SecretHashMap type
- [x] Write comprehensive tests (13 tests passing)
- [ ] Document migration plan
- [ ] Get team review/approval

### Phase 1 (Week 1)
- [x] Migrate DockerRegistrySettings.password
- [x] Update docker login handler
- [x] Migrate OnePassword::lookup_password
- [x] Update OnePassword callers
- [x] Migrate GitlabSettings.token
- [x] Migrate MattermostSettings.hook_id
- [x] Update notification handlers
- [x] Run full test suite (31 passed)
- [ ] Manual testing with real apps

### Phase 2 (Weeks 2-3)
- [ ] Migrate AppSettings.environment field
- [ ] Update resolve_environment_variables signature
- [ ] Update LoadBalancer implementations (Traefik, HAProxy)
- [ ] Update docker-compose config generation
- [ ] Update all API handlers
- [ ] Simplify SecureJson
- [ ] Run full test suite
- [ ] Manual API testing

### Phase 3 (Week 4)
- [ ] Add logging sanitization layer
- [ ] Security audit (grep for `.expose_secret()`)
- [ ] Performance testing
- [ ] Documentation updates
- [ ] Team training on MaskedSecret usage
- [ ] Final review and deployment

## Rollback Plan

If issues arise:
1. **Phase 1**: Easy rollback - only 2 files affected
2. **Phase 2**: Feature flag to toggle SecretHashMap vs HashMap
3. **Phase 3**: Logging layer can be disabled independently

## Success Criteria

- ✅ All existing tests pass
- ✅ No secrets visible in debug logs
- ✅ YAML serialization unchanged (docker-compose still works)
- ✅ API responses show partial masking
- ✅ No performance degradation (< 1% overhead)
- ✅ Code is simpler (less custom masking logic)
- ✅ Security: `.expose_secret()` calls are audited and justified

## Timeline

- **Week 1**: Phase 1 - Core secrets
- **Week 2-3**: Phase 2 - Environment variables
- **Week 4**: Phase 3 - Cleanup & optimization
- **Total**: 4 weeks

## Questions & Decisions

### Q: Should we migrate everything at once?
**A**: No, incremental migration reduces risk. Start with Phase 1 (core secrets).

### Q: What about backward compatibility?
**A**: Serialization is unchanged, so YAML files work as-is. No breaking changes.

### Q: Performance impact?
**A**: Minimal - SecretString has negligible overhead. Benchmarks needed but expected < 1%.

### Q: Can we keep SecureJson?
**A**: Yes, initially. Evaluate removal after Phase 2 when SecretHashMap is proven.

### Q: What if tests fail?
**A**: Rollback to previous phase. Each phase is independently testable.
