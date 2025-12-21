---
title: Add dedicated permissions for running blueprint actions (ActionRead/ActionWrite)
status: open
priority: 2
issue_type: feature
created_at: 2025-12-21T12:00:00.000000+00:00
updated_at: 2025-12-21T12:00:00.000000+00:00
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
        permission: action_read    # Safe - just generates a URL
        commands:
            cli:
                - drush uli --uri=$SCOTTY__PUBLIC_URL__NGINX

    drush:status:
        description: "Show Drupal status"
        permission: action_read    # Safe - read-only
        commands:
            cli:
                - drush status

    drush:cr:
        description: "Clear cache"
        permission: action_write   # Modifies state
        commands:
            cli:
                - drush cr

    drush:site-install:
        description: "Reinstall site (destructive)"
        permission: action_write   # Modifies state
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
    permissions: ['view', 'manage', 'action_read', 'action_write']

  viewer-plus:
    permissions: ['view', 'action_read']  # Can view + run safe actions only

  qa-tester:
    permissions: ['view', 'action_read', 'action_write']  # Actions but no app lifecycle
```

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

3. **Update custom action handler** in `scotty/src/api/rest/handlers/apps/custom_action.rs`:
   - Read the required permission from the action definition
   - Check user has this permission for the app's scope

4. **Update router** in `scotty/src/api/router.rs`:
   - Remove static `require_permission(Permission::Manage)` middleware
   - Permission check moves into the handler (dynamic based on action)

## Backwards Compatibility

- Actions without explicit `permission` field default to `ActionWrite`
- Existing roles with `Manage` permission do NOT automatically get action permissions
- Migration guide needed for updating role configurations

# Acceptance Criteria

- [ ] `ActionRead` and `ActionWrite` permissions added to Permission enum
- [ ] Action struct has optional `permission` field
- [ ] Custom action handler checks action-specific permission
- [ ] Default permission is `ActionWrite` when not specified
- [ ] Role configuration updated with examples
- [ ] Documentation updated for the permission system
- [ ] Existing blueprints work with default behavior

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
