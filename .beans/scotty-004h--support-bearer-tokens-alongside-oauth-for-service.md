---
# scotty-004h
title: Support bearer tokens alongside OAuth for service accounts
status: completed
type: feature
priority: high
created_at: 2025-12-21T12:44:46Z
updated_at: 2025-12-21T12:44:48Z
blocking:
    - scotty-631l
    - scotty-rnga
    - scotty-9qel
    - scotty-wjz6
---

# Description  Modify authentication middleware to allow bearer token authentication as a fallback when OAuth is enabled, enabling service accounts to use bearer tokens while users authenticate via OAuth.  # Design  Modify the auth middleware in scotty/src/api/basic_auth.rs to support fallback authentication when in OAuth mode: 1. When AuthMode::OAuth, first try OAuth validation via authorize_oauth_user_native 2. If OAuth validation fails, fallback to bearer token validation via authorize_bearer_user 3. This enables both OAuth tokens (for users) and bearer tokens (for service accounts) to work simultaneously 4. No breaking changes - Bearer and Development modes remain unchanged  # Acceptance Criteria  - OAuth tokens continue to validate successfully - Bearer tokens authenticate when OAuth validation fails - Bearer-only mode still works unchanged - Service accounts can authenticate with bearer tokens when OAuth is enabled - Authorization/RBAC works correctly for both token types - Tests cover OAuth priority and bearer fallback scenarios
