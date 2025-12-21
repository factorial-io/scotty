---
# scotty-4k76
title: Add dedicated permissions for running blueprint actions (ActionRead/ActionWrite)
status: todo
type: feature
priority: high
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T14:18:47Z
parent: scotty-d8n9
blocking:
    - scotty-0ry2
---

# Description

Add two new granular permissions specifically for executing blueprint actions, separate from the broader `Manage` permission. Following the existing `AdminRead`/`AdminWrite` naming pattern:

- `ActionRead` - for safe/read-only actions with no side effects
- `ActionWrite` - for actions that modify state

# Background

Blueprints define reusable application templates with associated actions that can be executed on running apps. There are two types of actions:

1. **Lifecycle actions** (built-in): `post_create`, `post_run`, `post_rebuild`
2. **Custom actions** (user-defined): e.g., `drush:uli`, database imports, cache clears

Example from `drupal-lagoon` blueprint:
```yaml
actions:
    post_create:
        commands:
            cli:
                - drush deploy --uri=$SCOTTY__PUBLIC_URL__NGINX
    drush:uli:
        description: "Generate a one-time login link"
        commands:
            cli:
                - drush uli --uri=$SCOTTY__PUBLIC_URL__NGINX
```

# Current Behavior

Running custom blueprint actions via `POST /api/v1/authenticated/apps/{app_name}/actions` requires `Permission::Manage`.

This is the same permission required for:
- Starting apps (`run`)
- Stopping apps (`stop`)
- Purging apps (`purge`)
- Rebuilding apps (`rebuild`)

# Problem

The current permission model is too coarse-grained:
- Users who should only run specific actions (e.g., generate login links, clear caches) must have full `Manage` permission
- This grants them access to stop, purge, or rebuild apps - which may not be appropriate for their role
- No way to allow "action runners" without giving them full app lifecycle control

# Proposed Solution

## New Permissions

```rust
pub enum Permission {
    View,
    Manage,
    Create,
    Destroy,
    Shell,
    Logs,
    ActionRead,   // NEW - safe/read-only actions (no side effects)
    ActionWrite,  // NEW - actions that modify state
    AdminRead,
    AdminWrite,
}
```

## Blueprint Configuration

Each action declares which permission it requires:

```yaml
actions:
    drush:uli:
        description: "Generate a one-time login link"
        permission: actionread    # Safe - just generates a URL
        commands:
            cli:
                - drush uli --uri=$SCOTTY__PUBLIC_URL__NGINX
    
    drush:status:
        description: "Show Drupal status"
        permission: actionread    # Safe - read-only
        commands:
            cli:
                - drush status
    
    drush:cr:
        description: "Clear cache"
        permission: actionwrite   # Modifies state
        commands:
            cli:
                - drush cr
    
    drush:site-install:
        description: "Reinstall site (destructive)"
        permission: actionwrite   # Modifies state
        commands:
            cli:
                - drush site-install -y
```

## Role Configuration

```yaml
roles:
  admin:
    permissions: ['*']
  
  developer:
    permissions: ['view', 'manage', 'actionread', 'actionwrite']
  
  viewer-plus:
    permissions: ['view', 'actionread']  # Can view + run safe actions only
  
  qa-tester:
    permissions: ['view', 'actionread', 'actionwrite']  # Actions but no app lifecycle
```

## Lifecycle Action Permissions

Lifecycle actions (`post_create`, `post_run`, `post_rebuild`) are **NOT** subject to action permissions. They:
- Execute automatically as part of app lifecycle state machine transitions
- Are already gated by the parent operation's permission (`Create` for post_create, `Manage` for post_run/post_rebuild)
- Cannot be invoked directly via the action API

This is intentional: if you have permission to create/run an app, the lifecycle hooks execute as part of that operation.

# Design

## Implementation Changes

1. **Add new permissions** in `scotty-core/src/authorization/permission.rs`:
   - Add `ActionRead` and `ActionWrite` variants to the Permission enum

2. **Add permission field to Action struct** in `scotty-core/src/settings/app_blueprint.rs`:
   ```rust
   pub struct Action {
       pub description: String,
       pub commands: HashMap<String, Vec<String>>,
       pub permission: Option<Permission>,  // NEW - defaults to ActionWrite
   }
   ```

3. **Add validation in AppBlueprint::validate()**:
   - Warn if lifecycle actions have permission field set (it will be ignored)
   - Validate permission field contains valid ActionRead/ActionWrite value

4. **Update custom action handler** in `scotty/src/api/rest/handlers/apps/custom_action.rs`:
   - Read the required permission from the action definition
   - Check user has this permission for the app's scope
   - Return 403 Forbidden with clear error message when permission denied

5. **Update router** in `scotty/src/api/router.rs`:
   - Remove static `require_permission(Permission::Manage)` middleware
   - Permission check moves into the handler (dynamic based on action)

## Breaking Change & Migration

**This is a breaking change.** Users with `Manage` permission can currently run all actions. After this change, they need explicit `actionread`/`actionwrite` permissions.

### Migration Steps

1. **Update role configurations** in `config/casbin/policy.yaml`:
   ```yaml
   # Before
   developer:
     permissions: ['view', 'manage', 'shell', 'logs', 'create']
   
   # After
   developer:
     permissions: ['view', 'manage', 'shell', 'logs', 'create', 'actionread', 'actionwrite']
   ```

2. **Add startup validation warning**: Log a warning at startup if any role has `manage` but not `actionread`/`actionwrite`:
   ```
   WARN: Role 'developer' has 'manage' permission but not 'actionread'/'actionwrite'. 
         Users with this role will not be able to run blueprint actions. 
         See migration guide: https://docs.scotty.dev/migration/0.3.0
   ```

3. **Document in CHANGELOG** as breaking change

### Why Not Auto-Grant?

We explicitly chose NOT to have `Manage` automatically grant action permissions because:
- It would hide the new permission model from administrators
- Roles should be explicit about capabilities granted
- The startup warning provides clear migration guidance

# Security Considerations

## Blueprint Trust Boundary

**Important**: The `actionread`/`actionwrite` distinction is **developer-declared, not enforced**. 

- Blueprint authors mark actions as "safe" or "state-modifying"
- The server trusts these declarations
- A malicious or mistaken blueprint could mark a destructive action as `actionread`

This is acceptable because:
- Blueprints are server configuration, not user input
- Only administrators with file system access can modify blueprints
- The permission system controls WHO can run actions, not WHAT actions do

Document this trust boundary clearly in the authorization docs.

# Acceptance Criteria

- [ ] `ActionRead` and `ActionWrite` permissions added to Permission enum
- [ ] `Permission::all()` includes new permissions
- [ ] `Permission::as_str()` and `from_str()` handle new permissions (lowercase format)
- [ ] Action struct has optional `permission` field
- [ ] Custom action handler checks action-specific permission
- [ ] Handler returns 403 with clear error message on permission denied
- [ ] Default permission is `ActionWrite` when not specified
- [ ] Lifecycle actions documented as not subject to action permissions
- [ ] Validation warns if lifecycle actions have permission field
- [ ] Startup warning for roles with `manage` but no action permissions
- [ ] Role configuration examples updated
- [ ] OpenAPI schema updated with permission requirements
- [ ] TypeScript bindings regenerated
- [ ] Documentation updated for the permission system
- [ ] CHANGELOG documents breaking change
- [ ] Migration guide written

# Test Requirements

## Unit Tests
- [ ] Permission enum serialization/deserialization (lowercase format)
- [ ] Action struct deserialization with/without permission field
- [ ] Default permission is ActionWrite when omitted

## Integration Tests
- [ ] User with ActionRead can run actionread actions
- [ ] User with ActionRead cannot run actionwrite actions (403)
- [ ] User with ActionWrite can run actionwrite actions
- [ ] User with Manage but no action permissions cannot run actions (403)
- [ ] Lifecycle actions execute without action permissions (via Manage)

## Validation Tests
- [ ] Blueprint validation warns on lifecycle action with permission field

# Use Cases

1. **QA team members** can run test data imports without being able to stop production apps
2. **Developers** can generate login links or clear caches without full app control
3. **CI/CD pipelines** can run deployment actions with minimal required permissions
4. **View-only users** can run safe diagnostic actions (status checks) without any write access

# Related Files

- Permission enum: `scotty-core/src/authorization/permission.rs`
- Action struct: `scotty-core/src/settings/app_blueprint.rs`
- Router configuration: `scotty/src/api/router.rs`
- Custom action handler: `scotty/src/api/rest/handlers/apps/custom_action.rs`
- Authorization middleware: `scotty/src/api/middleware/authorization.rs`
