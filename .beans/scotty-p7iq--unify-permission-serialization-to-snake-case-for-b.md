---
# scotty-p7iq
title: Unify permission serialization to lowercase (keep serde format, update Casbin)
status: todo
type: task
priority: critical
created_at: 2025-12-21T14:02:36Z
updated_at: 2025-12-21T14:17:49Z
parent: scotty-d8n9
blocking:
    - scotty-4k76
    - scotty-0ry2
---

# Description

Currently there's an inconsistency in permission serialization:
- Serde uses `#[serde(rename_all = "lowercase")]` → serializes as `adminread`, `actionread`
- Casbin `as_str()` uses snake_case → `admin_read`, `action_read`

**Decision:** Keep serde's lowercase format as the canonical format. Update Casbin to accept both formats for backwards compatibility, but only write the new lowercase format.

## Changes Required

1. **Keep serde attribute unchanged** in `scotty-core/src/authorization/permission.rs`:
   ```rust
   #[serde(rename_all = "lowercase")]  // Keep as-is
   pub enum Permission {
   ```

2. **Update `as_str()`** to return lowercase (no underscore):
   ```rust
   pub fn as_str(&self) -> &'static str {
       match self {
           Permission::View => "view",
           Permission::Manage => "manage",
           Permission::AdminRead => "adminread",   // Changed from admin_read
           Permission::AdminWrite => "adminwrite", // Changed from admin_write
           Permission::ActionRead => "actionread",
           Permission::ActionWrite => "actionwrite",
           // ... etc
       }
   }
   ```

3. **Update `from_str()`** to accept BOTH formats (backwards compatibility):
   ```rust
   pub fn from_str(s: &str) -> Option<Permission> {
       match s.to_lowercase().as_str() {
           // New format (lowercase)
           "adminread" => Some(Permission::AdminRead),
           "adminwrite" => Some(Permission::AdminWrite),
           "actionread" => Some(Permission::ActionRead),
           "actionwrite" => Some(Permission::ActionWrite),
           // Legacy format (snake_case) - for backwards compatibility
           "admin_read" => Some(Permission::AdminRead),
           "admin_write" => Some(Permission::AdminWrite),
           "action_read" => Some(Permission::ActionRead),
           "action_write" => Some(Permission::ActionWrite),
           // ... etc
       }
   }
   ```

4. **Update documentation** to use lowercase format in all examples

5. **Regenerate TypeScript bindings** via ts-generator

## Backwards Compatibility

- Existing `policy.yaml` files using `admin_read` will continue to work
- New configurations should use `adminread` format
- Log a deprecation warning when legacy format is detected

## Acceptance Criteria

- [ ] `as_str()` returns lowercase format (no underscores)
- [ ] `from_str()` accepts both lowercase and snake_case formats
- [ ] Deprecation warning logged when snake_case format is used
- [ ] All YAML examples in documentation use lowercase: `adminread`, `adminwrite`, `actionread`, `actionwrite`
- [ ] Frontend updated to handle lowercase permission names
- [ ] TypeScript bindings regenerated
- [ ] All tests pass

## Note

This is a minor breaking change for external API consumers expecting snake_case in JSON responses, but existing YAML configurations will continue to work.
