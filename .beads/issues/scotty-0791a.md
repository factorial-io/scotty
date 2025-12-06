---
title: Update auth:status to check token validity and return non-zero on failure
status: closed
priority: 3
issue_type: task
created_at: 2025-12-06T14:28:43.340602+00:00
updated_at: 2025-12-06T14:39:44.377073+00:00
closed_at: 2025-12-06T14:39:44.377072+00:00
---

# Description

Location: scottyctl/src/commands/auth.rs:146-190

NEW SIMPLIFIED APPROACH (no dependency on scotty-46245):

Changes needed:
1. For authenticated users (OAuth or Bearer), validate token BEFORE calling display_user_permissions
2. Make a lightweight API call (e.g., GET /scopes/list) to check if token is valid
3. If API returns 401/403, token is invalid:
   - Print error message suggesting re-auth
   - Return Err() to exit with code 1
   - Skip display_user_permissions entirely
4. If API succeeds, proceed normally with display_user_permissions

Error messages:
- OAuth: 'Authentication token expired or invalid. Run scottyctl --server <url> auth:login to re-authenticate'
- Bearer: 'Bearer token invalid. Please update SCOTTY_ACCESS_TOKEN environment variable'

Use anyhow::bail! or return Err(anyhow::anyhow!(...)) to ensure exit code 1

This is simpler than refactoring display_user_permissions - we just validate earlier and skip it when invalid.

Part of: scotty-28453
