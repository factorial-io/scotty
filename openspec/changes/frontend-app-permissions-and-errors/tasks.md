## 1. Server

- [ ] 1.1 In `scotty/src/api/middleware/authorization.rs`, return `AppError::ScopeAccessDenied(reason).into_response()` on refusal instead of `Err(StatusCode::FORBIDDEN)`; verify an existing or new middleware test asserts status 403 and a JSON body whose `message` contains `lacks manage permission`

## 2. Frontend permissions

- [ ] 2.1 In `frontend/src/stores/permissionStore.ts`, make `hasPermission` resolve the app's scopes from `appsStore` (`settings.scopes`, default `['default']`, `_global` and unknown apps keep any-scope semantics) and grant only from matching `ScopeInfo` entries; the frontend has no test runner, so verify with `bun run check` plus the manual check in 4.2 (held in another scope only → hidden, held in the app's scope → shown, dev mode → shown)
- [ ] 2.2 Remove the debug `console.log` blocks in `permissionStore.ts` and the app detail page that dump permission state; verify `bun run lint` passes

## 3. Frontend error reporting

- [ ] 3.1 In `frontend/src/lib/index.ts`, extract `message` from a `{ error, message }` JSON body on non-2xx responses and throw it; verify by the manual check in 4.2 that the alert shows the server's `message`, and by `bun run check`
- [ ] 3.2 App detail page (`routes/dashboard/[slug]/+page.svelte`): wrap `dispatchAppCommand` in try/catch/finally, reset `current_action`, render an `alert alert-error` with the message below the action buttons; verify `bun run check`
- [ ] 3.3 `components/start-stop-app-action.svelte`: same handling with an error tooltip on the button, matching the existing failed-task tooltip; verify `bun run check`
- [ ] 3.4 `components/custom-actions-dropdown.svelte`: surface the caught error instead of only logging it; verify `bun run check`

## 4. Wrap-up

- [ ] 4.1 `cd frontend && bun run check && bun run lint`, `cargo test -p scotty --lib api::middleware`; verify all green
- [ ] 4.2 Manual check against the local server: as a user with `manage` only on `client-a`, open a default-scope app; verify no Run/Rebuild buttons are shown, and that forcing the request (e.g. `curl` with the session cookie) returns 403 with the JSON message
- [ ] 4.3 Add a kenkeep node under `frontend/` describing per-app permission resolution and inline action errors, run `npx kenkeep index rebuild`; create a bean, complete it with a summary, commit with `jj` as `fix(frontend): gate app actions per scope and show refused actions`
