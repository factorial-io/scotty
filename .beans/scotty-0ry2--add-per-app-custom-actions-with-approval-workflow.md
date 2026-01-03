---
# scotty-0ry2
title: Add per-app custom actions with approval workflow
status: todo
type: feature
priority: high
created_at: 2025-12-21T12:44:46Z
updated_at: 2025-12-21T14:20:24Z
parent: scotty-d8n9
---

# Description

Allow users to create custom actions for individual apps via scottyctl, with an optional approval workflow for security. This extends the blueprint action system to support ad-hoc, per-app actions without modifying blueprints.

# Background

Currently, actions are defined at the blueprint level and shared across all apps using that blueprint. This proposal allows:

- Creating app-specific actions via CLI
- Multi-line command support
- Security through an approval workflow
- Audit trail for all action operations

# New Permissions

Extends the permission model from scotty-4k76 with granular action management permissions:

```rust
pub enum Permission {
    // ... existing
    ActionRead,     // Execute actions marked as actionread
    ActionWrite,    // Execute actions marked as actionwrite
    ActionCreate,   // NEW: Create custom actions (pending status)
    ActionList,     // NEW: List custom actions for apps in scope
    ActionDelete,   // NEW: Delete custom actions in scope
    ActionApprove,  // NEW: Approve/reject/revoke pending actions
}
```

## Permission Matrix

| Permission | Capabilities |
|------------|--------------|
| `actionread` | Execute actions marked as `actionread` |
| `actionwrite` | Execute actions marked as `actionwrite` (does NOT imply actionread) |
| `actioncreate` | Create custom actions for apps in user's scope (created as pending) |
| `actionlist` | List custom actions - sees own pending + all approved actions |
| `actiondelete` | Delete any custom action in user's scope |
| `actionapprove` | Approve/reject/revoke pending actions, list all pending |

**Note:** `actionwrite` does NOT imply `actionread`. They are independent permissions. A user who can clear caches (`actionwrite`) doesn't automatically get to generate login URLs (`actionread`).

## Self-Approval

Users with `actionapprove` CAN approve their own created actions. This is intentional for:
- Smaller teams where separation of duties is unnecessary overhead
- Development environments where speed matters
- Administrators who are trusted anyway

For stricter environments, use organizational policy rather than technical enforcement.

# Storage

Custom actions are stored in the existing `AppSettings` struct, extending it with a new field:

```rust
// In scotty-core/src/settings/app_settings.rs
pub struct AppSettings {
    // ... existing fields
    
    /// Custom actions defined for this specific app (not from blueprint)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom_actions: HashMap<String, CustomAction>,
}
```

This approach:
- Follows existing patterns for app-specific configuration
- Persists with existing app settings serialization
- Works with backup/restore functionality
- No new dependencies required

Storage location: `{apps_directory}/{app_name}/settings.yaml`

# CLI Commands

## Creating Actions

```bash
# From YAML file (recommended for multi-line)
scottyctl app:action:create my-app --from-file action.yaml

# Multiple --command flags
scottyctl app:action:create my-app \
  --name "deploy" \
  --description "Run deployment" \
  --service cli \
  --permission actionwrite \
  --command "drush cr" \
  --command "drush updb -y" \
  --command "drush cim -y"

# From stdin (heredoc)
scottyctl app:action:create my-app \
  --name "deploy" \
  --service cli \
  --commands - <<'EOF'
drush cr
drush updb -y
drush cim -y
EOF
```

**action.yaml format**:
```yaml
name: deploy
description: Run full deployment
permission: actionwrite
commands:
  cli:
    - drush cr
    - drush updb -y
    - drush cim -y
    - drush deploy:hook -y
  nginx:
    - nginx -s reload
```

## Listing Actions (actionlist permission)

```bash
# List actions for an app (shows own pending + all approved)
scottyctl app:action:list my-app

# Output:
# NAME       STATUS    PERMISSION    CREATED BY           CREATED AT
# deploy     pending   actionwrite   dev@factorial.io     2025-12-21 12:00
# drush:uli  approved  actionread    admin@factorial.io   2025-12-20 10:00
```

## Deleting Actions (actiondelete permission)

```bash
# Delete any action in scope
scottyctl app:action:delete my-app deploy
```

## Approval Workflow (actionapprove permission)

```bash
# List all pending actions across all apps in scope
scottyctl admin:actions:pending

# Output:
# APP        NAME      PERMISSION    CREATED BY           COMMANDS
# my-app     deploy    actionwrite   dev@factorial.io     drush cr; drush updb...
# other-app  migrate   actionwrite   other@factorial.io   php artisan migrate

# Show full action details for review
scottyctl admin:actions:show my-app deploy

# Approve action
scottyctl admin:actions:approve my-app deploy --comment "Looks good"

# Reject action
scottyctl admin:actions:reject my-app deploy --comment "curl not allowed"

# Revoke previously approved action
scottyctl admin:actions:revoke my-app deploy --comment "No longer needed"
```

# Approval Workflow

## State Machine

```
┌──────────┐    create     ┌─────────┐
│          │──────────────▶│         │
│  (none)  │               │ pending │
│          │               │         │
└──────────┘               └────┬────┘
                                │
                 ┌──────────────┼──────────────┐
                 │              │              │
              approve        reject         expire
                 │              │              │
                 ▼              ▼              ▼
            ┌────────┐    ┌──────────┐   ┌─────────┐
            │approved│    │ rejected │   │ expired │
            └────────┘    └──────────┘   └─────────┘
                 │
              revoke
                 │
                 ▼
            ┌────────┐
            │revoked │
            └────────┘
```

## Workflow Example

1. Developer creates action:
   ```
   $ scottyctl app:action:create my-app --from-file deploy.yaml
   → Action 'deploy' created (status: pending)
   → Notification sent to #scotty-approvals
   ```

2. Admin reviews:
   ```
   $ scottyctl admin:actions:pending
   $ scottyctl admin:actions:show my-app deploy
   $ scottyctl admin:actions:approve my-app deploy --comment "LGTM"
   → Action status: approved
   ```

3. Developer can now run:
   ```
   $ scottyctl app:action:run my-app deploy
   → Executes if user has actionwrite permission
   ```

4. Admin revokes if needed:
   ```
   $ scottyctl admin:actions:revoke my-app deploy --comment "Security concern"
   → Action no longer executable
   ```

# Design

## Data Model

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActionStatus {
    Pending,
    Approved,
    Rejected,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAction {
    pub name: String,
    pub description: String,
    pub commands: HashMap<String, Vec<String>>,
    pub permission: Permission,  // actionread or actionwrite
    
    // Audit fields
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    
    // Approval workflow
    pub status: ActionStatus,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_comment: Option<String>,
    
    // Expiration
    pub expires_at: Option<DateTime<Utc>>,
}
```

## Validation

When creating a custom action, validate:
1. Action name is unique for this app
2. All target services exist in the app's blueprint `required_services`
3. Commands pass blocklist validation
4. Command length doesn't exceed `max_command_length`
5. App doesn't exceed `max_actions_per_app` limit

## API Endpoints

### User Endpoints

| Method | Endpoint | Permission | Description |
|--------|----------|------------|-------------|
| POST | `/api/v1/authenticated/apps/{app}/custom-actions` | actioncreate | Create action |
| GET | `/api/v1/authenticated/apps/{app}/custom-actions` | actionlist | List actions |
| DELETE | `/api/v1/authenticated/apps/{app}/custom-actions/{name}` | actiondelete | Delete action |

### Admin Endpoints

| Method | Endpoint | Permission | Description |
|--------|----------|------------|-------------|
| GET | `/api/v1/authenticated/admin/actions/pending` | actionapprove | List all pending |
| GET | `/api/v1/authenticated/admin/actions/{app}/{name}` | actionapprove | Show details |
| PATCH | `/api/v1/authenticated/admin/actions/{app}/{name}` | actionapprove | Update status |

**PATCH body for status transitions:**
```json
{
  "status": "approved",
  "comment": "LGTM"
}
```

Valid status transitions via PATCH: `pending` → `approved`/`rejected`, `approved` → `revoked`

## Security Controls

### Layered Security Model

```
┌─────────────────────────────────────────────────────────────┐
│                    Layer 1: Permission                       │
│         Only users with actioncreate can create              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  Layer 2: Approval Workflow                  │
│         Actions require approval before execution            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Layer 3: Blocklist                        │
│         Server-wide blocked patterns (glob matching)         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  Layer 4: Rate Limiting                      │
│         Max N action creations per user per hour             │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Layer 5: Audit Trail                       │
│      Log all action CRUD with user, IP, timestamp            │
└─────────────────────────────────────────────────────────────┘
```

### Command Blocklist

Uses **glob pattern matching** (via the `glob` crate or similar). Patterns are matched against each command line.

```yaml
action_security:
  blocked_patterns:
    - "curl *"        # Blocks: curl anything
    - "wget *"        # Blocks: wget anything
    - "nc *"          # Blocks: netcat
    - "bash -i*"      # Blocks: interactive bash
    - "sh -i*"        # Blocks: interactive sh
    - "python -c*"    # Blocks: inline python
    - "perl -e*"      # Blocks: inline perl
    - "rm -rf /*"     # Blocks: recursive delete from root
    - "*; curl*"      # Blocks: command chaining with curl
    - "*&& wget*"     # Blocks: command chaining with wget
  
  max_command_length: 500
  max_actions_per_app: 20
```

**Matching behavior:**
- Each command in the action is checked against all patterns
- `*` matches any sequence of characters (standard glob)
- Matching is case-sensitive
- If ANY command matches ANY pattern, creation is rejected with error listing the matched pattern

**Multi-line command checking:**
- Each line is checked independently
- All lines must pass validation

### Rate Limiting

```yaml
action_security:
  rate_limits:
    creations_per_user_per_hour: 10
    scope: per_user  # Options: global, per_user, per_app, per_user_per_app
```

Rate limit violations return 429 Too Many Requests with `Retry-After` header.

## Expiration Behavior

- **Pending actions**: Auto-expire after `pending_ttl_days` (default: 7)
- **Approved actions**: Auto-expire after `approved_ttl_days` (0 = never)
- **Expired during execution**: Action completes if already started, but cannot be started again
- **Re-approval**: Expired actions cannot be re-approved; must be recreated

Background task runs periodically to transition expired actions.

## Configuration

```yaml
# Server config
action_approval:
  # Enable custom actions (default: false)
  enabled: true
  
  # Require approval (can be disabled for dev environments)
  require_approval: true
  
  # Notifications for pending actions
  notify_on_pending:
    - mattermost:
        webhook_url: https://mm.example.com/hooks/xxx
        channel: "#scotty-approvals"
  
  # Auto-expire pending actions after N days
  pending_ttl_days: 7
  
  # Auto-expire approved actions after N days (0 = never)
  approved_ttl_days: 90
```

## Role Configuration Example

```yaml
roles:
  admin:
    permissions: ['*']
  
  lead-developer:
    permissions:
      - view
      - manage
      - actionread
      - actionwrite
      - actioncreate
      - actionlist
      - actiondelete
      - actionapprove   # Can approve for their scopes
  
  developer:
    permissions:
      - view
      - manage
      - actionread
      - actionwrite
      - actioncreate
      - actionlist      # Can see actions, but needs approval
  
  viewer:
    permissions:
      - view
      - actionread      # Can run safe actions only
      - actionlist
```

# Implementation Phases

## Phase 1: Core Infrastructure
- [ ] Add `ActionCreate`, `ActionList`, `ActionDelete`, `ActionApprove` permissions
- [ ] Create `CustomAction` struct with status tracking
- [ ] Extend `AppSettings` to store custom actions
- [ ] Add CRUD API endpoints for custom actions
- [ ] Add service validation (target services must exist in blueprint)

## Phase 2: Approval Workflow
- [ ] Implement action status state machine
- [ ] Add admin approval endpoints (PATCH for status transitions)
- [ ] Add notification integration for pending actions
- [ ] Implement auto-expiration background task

## Phase 3: CLI Integration
- [ ] Add `app:action:create` with multi-line support (file, flags, stdin)
- [ ] Add `app:action:list` and `app:action:delete`
- [ ] Add `admin:actions:pending/show/approve/reject/revoke`

## Phase 4: Security Hardening
- [ ] Implement command blocklist validation (glob matching)
- [ ] Add rate limiting for action creation
- [ ] Add comprehensive audit logging

# Acceptance Criteria

- [ ] All 4 new permissions added to Permission enum
- [ ] `CustomAction` struct with all required fields
- [ ] `AppSettings` extended with `custom_actions` field
- [ ] CRUD endpoints for custom actions
- [ ] Service validation on action creation
- [ ] Users with `actioncreate` can create actions (pending status)
- [ ] Users with `actionapprove` can approve/reject pending actions
- [ ] Self-approval works (same user can create and approve)
- [ ] Only approved actions can be executed
- [ ] Multi-line commands supported via file or stdin
- [ ] Blocklist validation rejects matching commands (glob patterns)
- [ ] Rate limiting enforced on action creation
- [ ] Audit trail captures all action CRUD operations
- [ ] Notifications sent for pending actions (if configured)
- [ ] Actions auto-expire based on TTL configuration
- [ ] Expired actions cannot be re-approved
- [ ] OpenAPI schema updated
- [ ] TypeScript bindings regenerated

# Test Requirements

## Unit Tests
- [ ] Permission enum serialization (lowercase format)
- [ ] CustomAction serialization/deserialization
- [ ] ActionStatus state machine transitions
- [ ] Blocklist glob pattern matching
- [ ] Command validation logic

## Integration Tests
- [ ] User with actioncreate can create action (pending)
- [ ] User without actioncreate gets 403
- [ ] User with actionapprove can approve actions
- [ ] Self-approval works
- [ ] User with actionlist sees own pending + all approved
- [ ] User with actiondelete can delete any action in scope
- [ ] Blocked commands rejected at creation time
- [ ] Rate limiting returns 429
- [ ] Expired actions cannot be executed

## Validation Tests
- [ ] Target services must exist in blueprint
- [ ] Action name uniqueness per app
- [ ] Max actions per app limit

# Related Files

- Permission enum: `scotty-core/src/authorization/permission.rs`
- App settings: `scotty-core/src/settings/app_settings.rs`
- Router: `scotty/src/api/router.rs`
- Notification service: `scotty/src/notification/`
