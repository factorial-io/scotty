---
# scotty-n1du
title: Gate app actions per scope and show refused actions (frontend-app-permissions-and-errors)
status: completed
type: bug
priority: normal
created_at: 2026-09-05T12:33:06Z
updated_at: 2026-09-05T13:25:52Z
---

Implement OpenSpec change frontend-app-permissions-and-errors: per-app permission checks, inline action errors, JSON 403 body from the authorization middleware.

## Summary of Changes

- Authorization middleware now returns 403 with a JSON `{error, message}` body (`AppError::ScopeAccessDenied`) instead of an empty response; integration test in `scotty/tests/test_authorization_denied_body.rs`.
- `hasPermission` resolves an app's scopes from `appsStore` (`settings.scopes`, default `['default']`) and only grants from matching user scopes; unknown apps and `_global` keep any-scope fallback. Added `scopes` to the frontend `AppSettings` type. Vitest coverage extended.
- `authenticatedApiCall` throws the server's `message` on non-2xx JSON bodies, falls back to status text otherwise.
- Detail page, start/stop button and custom-actions dropdown catch dispatch errors and show them in a global error dialog (errorStore + error-dialog.svelte in the root layout), resetting busy state. Permission checks take the App object so a direct detail-page load does not fall back to any-scope.
- Removed permission debug `console.log` blocks. kenkeep node added under frontend/.
- Manual check (task 4.2) not done in this session.
