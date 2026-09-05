---
type: map
title: Frontend gates app actions per scope and shows refused dispatches inline
description: >-
  hasPermission(appName, perm) resolves the app's scopes from appsStore
  (settings.scopes, default ['default']) and grants only from matching user
  scopes; unknown apps and `_global` keep any-scope semantics. Failed action
  dispatches surface the server's `{error, message}` text inline (alert on the
  detail page, error tooltip on the list button) instead of dying silently.
tags:
  - frontend
  - permissions
  - svelte
  - errors
kk_schema_version: 3
kk_id: map-frontend-per-app-permissions-and-action-errors
kk_derived_from: []
kk_relates_to:
  - map-root-layout-loads-user-permissions-when-the-user-is-logged-in
kk_depends_on: []
kk_confidence: high
---
`frontend/src/stores/permissionStore.ts` `hasPermission(app | name, permission)` takes the `App` where the caller has it (detail page `data`, list rows), or looks a name up in `appsStore.apps`. Pass the `App` on the detail page: a direct page load leaves the apps store empty, and an unknown name would fall back to "any scope" and show buttons the server refuses. The app's scopes are `settings?.scopes ?? ['default']` and a permission is granted only if one of the user's scopes with that name lists it (or `*`). Apps not in the store, and the `_global` pseudo-app used by `hasAdminPermission`, fall back to "granted in any scope". Dev auth mode grants everything. The visible buttons therefore match what the server's Casbin check will allow; a mismatch means the app list is stale, not a permission bug.

Errors from `authenticatedApiCall` (`frontend/src/lib/index.ts`) carry the server's `message` from the `{ error: true, message }` body on non-2xx responses (the authorization middleware returns `AppError::ScopeAccessDenied` JSON on 403); non-JSON bodies fall back to `API call failed: <status> <statusText>`. Dispatch sites (detail page, `start-stop-app-action.svelte`, `custom-actions-dropdown.svelte`) catch and call `showError(context, err)` from `stores/errorStore.ts`; `components/error-dialog.svelte` (native `<dialog>` with daisyUI `modal`) is mounted once in `routes/+layout.svelte` and opens whenever the store holds a message. Busy state is reset on failure so the button is usable again.
