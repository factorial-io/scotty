---
title: Refactor display_user_permissions to return Result indicating token validity
status: closed
priority: 3
issue_type: task
created_at: 2025-12-06T14:28:39.393108+00:00
updated_at: 2025-12-06T14:35:49.606347+00:00
closed_at: 2025-12-06T14:35:49.606347+00:00
---

# Description

OBSOLETE - Simplified approach: validate token before calling display_user_permissions

Original plan was to refactor display_user_permissions to return Result<bool>, but we can skip calling it entirely when token is invalid.

New approach (see updated scotty-0791a):
- Make a lightweight API call to validate token (e.g., GET /scopes/list)
- If 401/403, token is invalid - skip display_user_permissions and return Err
- If success, call display_user_permissions as normal
- Much simpler than refactoring display_user_permissions

Marking this task as obsolete. Will close when new approach is implemented.

Part of: scotty-28453
