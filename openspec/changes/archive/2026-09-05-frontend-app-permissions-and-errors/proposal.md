## Why

The frontend shows an app's action buttons (Run, Stop, Rebuild, Purge, Destroy, custom actions) whenever the user holds the permission in *any* of their scopes, because `hasPermission` ignores the app. When the server then denies the action for that app's scope, the click handler has no error handling: the request fails with 403, the button stays in its busy state, and nothing is shown to the user. Today this happens for every default-scope app when a user only has `manage` on a client scope.

## What Changes

- Per-app permission checks in the frontend: a permission is granted for an app only if the user holds it in one of that app's scopes (`settings.scopes`, or `default` when the app has no settings). Global checks (admin pages) keep the any-scope semantics.
- Action dispatch on the app detail page, the app list start/stop button and the custom actions dropdown catches API errors, resets the busy state and shows the server's message inline.
- `authenticatedApiCall` includes the server's `message` in the thrown error when the response body is the standard `{ error, message }` shape.
- The authorization middleware returns the standard JSON error body on 403 (`{ "error": true, "message": "Access denied: ... lacks manage permission" }`) instead of an empty body, so the frontend and scottyctl can show the reason.

## Capabilities

### New Capabilities

- `app-action-feedback`: which app actions the frontend offers for an app, and how a refused or failed action request is reported to the user.

### Modified Capabilities

None.

## Impact

- `frontend/src/stores/permissionStore.ts`: `hasPermission` and `getAppPermissions` resolve the app's scopes from the apps store.
- `frontend/src/routes/dashboard/[slug]/+page.svelte`, `frontend/src/components/start-stop-app-action.svelte`, `frontend/src/components/custom-actions-dropdown.svelte`: try/catch around dispatch, inline error display.
- `frontend/src/lib/index.ts`: error message extraction.
- `scotty/src/api/middleware/authorization.rs`: JSON body on 403. Response status is unchanged, so scottyctl and existing tests keep working; scottyctl already prints the body's `message` when present.
