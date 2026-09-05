## Context

See proposal.md. The frontend loads the user's scopes with their permissions from `GET scopes/list` into `permissionStore`; the apps list (`appsStore`) already carries each app's `settings.scopes` (server default `["default"]`). The server resolves an app's scope in `inspect_app` and falls back to `default` when the declared scopes do not exist. The authorization middleware currently returns a bare `StatusCode::FORBIDDEN`; `AppError` responses use `{ "error": true, "message": ... }`, which the frontend's `ApiError` type already models.

## Goals / Non-Goals

**Goals:**
- Buttons match what the server will allow for that app, using data the frontend already has.
- No action request can fail silently.
- One error shape from the server for every refused request.

**Non-Goals:**
- A toast/notification system. Errors are shown inline where the action was triggered.
- Changing server-side authorization rules or the scope model.
- Hiding apps the user cannot manage from the list (they stay visible with view permission).

## Decisions

### D1: Resolve app scopes client-side from the apps store

`hasPermission(appName, permission)` keeps its signature. It looks the app up in `appsStore` and uses `app.settings?.scopes ?? ['default']`; the permission is granted if any of the user's `ScopeInfo` entries with a matching name lists the permission or `*`. If the app is not in the store yet (detail page opened directly before the list loaded), it falls back to the previous any-scope behavior so the page is not needlessly empty; the server remains authoritative and D3 shows any refusal.

Alternative: a new endpoint returning per-app effective permissions. Rejected for now: it adds a round trip per app and duplicates data the client has; revisit if the client-side rule diverges from the server's (e.g. when invalid scopes fall back to `default`, which the client cannot see; that case now surfaces as a displayed 403 instead of a silent one).

### D2: Global checks stay scope-agnostic

`hasAdminPermission` and other callers passing `_global` keep the any-scope semantics; the server's middleware uses the same rule for routes without an app in the path.

### D3: Inline error state per dispatch site

Each of the three dispatch sites (detail page, list start/stop button, custom actions dropdown) gets a local `actionError: string | null`, set in a `catch`, cleared on the next click, rendered as a DaisyUI `alert alert-error` (detail page) or an error tooltip (list button, matching the existing failed-task tooltip). `current_action`/`task_id`/`currentAction` are reset in `finally`.

Alternative: a global error store and toast component. More code and a new UI pattern for three call sites; not warranted yet.

### D4: `authenticatedApiCall` surfaces the server message

On a non-2xx response it tries `response.json()`; if the body matches `{ error: true, message }` the thrown `Error` carries `message`, otherwise `"API call failed: <status> <statusText>"` as today. 401 handling is unchanged.

### D5: Middleware 403 uses `AppError`

The authorization middleware maps a refusal to `AppError::ScopeAccessDenied(reason)` (already `FORBIDDEN`) via `.into_response()` instead of `Err(StatusCode::FORBIDDEN)`, so the body is the standard JSON. The `reason` is the existing log text without the user email.

## Risks / Trade-offs

- [No frontend unit test runner exists] → The permission rule is small and pure; verification is `bun run check` plus a manual check with a scoped user. Adding Vitest is out of scope here.

- [Client and server disagree on an app's scope (invalid scopes → server falls back to `default`)] → Buttons may be shown that the server refuses; the refusal is now displayed with its reason, which is the failure mode we want.
- [Detail page opened before the apps list is loaded] → Fallback to any-scope semantics (D1); the list is loaded by the layout on login, so this is rare.
- [Changing the 403 body] → Status is unchanged; scottyctl already prefers the JSON `message` when present. Middleware unit tests asserting on status keep passing.
