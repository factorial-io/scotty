---
title: Support bearer tokens alongside OAuth for service accounts
status: closed
priority: 2
issue_type: feature
assignee: claude
created_at: 2025-11-03T10:07:22.572676+00:00
updated_at: 2025-11-24T20:17:25.584399+00:00
closed_at: 2025-11-03T10:17:22.810051+00:00
---

# Description

Modify authentication middleware to allow bearer token authentication as a fallback when OAuth is enabled, enabling service accounts to use bearer tokens while users authenticate via OAuth.

# Design

Modify the auth middleware in scotty/src/api/basic_auth.rs to support fallback authentication when in OAuth mode:
1. When AuthMode::OAuth, first try OAuth validation via authorize_oauth_user_native
2. If OAuth validation fails, fallback to bearer token validation via authorize_bearer_user
3. This enables both OAuth tokens (for users) and bearer tokens (for service accounts) to work simultaneously
4. No breaking changes - Bearer and Development modes remain unchanged

# Acceptance Criteria

- OAuth tokens continue to validate successfully
- Bearer tokens authenticate when OAuth validation fails
- Bearer-only mode still works unchanged
- Service accounts can authenticate with bearer tokens when OAuth is enabled
- Authorization/RBAC works correctly for both token types
- Tests cover OAuth priority and bearer fallback scenarios
