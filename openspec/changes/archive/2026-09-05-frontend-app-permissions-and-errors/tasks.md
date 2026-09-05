## 1. Server

- [x] 1.1 In `scotty/src/api/middleware/authorization.rs`, return `AppError::ScopeAccessDenied(reason).into_response()` on refusal instead of `Err(StatusCode::FORBIDDEN)`; verify an existing or new middleware test asserts status 403 and a JSON body whose `message` contains `lacks manage permission`

## 2. Frontend permissions

- [x] 2.1 In `frontend/src/stores/permissionStore.ts`, make `hasPermission` resolve the app's scopes from `appsStore` (`settings.scopes`, default `['default']`, `_global` and unknown apps keep any-scope semantics) and grant only from matching `ScopeInfo` entries; verify Vitest tests in `permissionStore.test.ts` cover: held in another scope only → false, held in the app's scope → true, app without settings → `default`, unknown app → any-scope fallback, `_global` → any scope, dev mode → true
- [x] 2.2 Remove the debug `console.log` blocks in `permissionStore.ts` and the app detail page that dump permission state; verify `bun run lint` passes

## 3. Frontend error reporting

- [x] 3.1 In `frontend/src/lib/index.ts`, extract `message` from a `{ error, message }` JSON body on non-2xx responses and throw it; verify Vitest tests in `lib/index.test.ts`: 403 with `{error, message}` throws the message, non-JSON body throws `API call failed: 403 Forbidden`
- [x] 3.2 App detail page (`routes/dashboard/[slug]/+page.svelte`): wrap `dispatchAppCommand` in try/catch, reset `current_action`, show the message via the global error dialog (`stores/errorStore.ts` + `components/error-dialog.svelte` mounted in the root layout); pass the `App` (not its name) to `getAppPermissions` so a direct page load does not fall back to any-scope; verify `bun run check`
- [x] 3.3 `components/start-stop-app-action.svelte`: take the `App` as prop, same catch → error dialog; verify `bun run check`
- [x] 3.4 `components/custom-actions-dropdown.svelte`: surface the caught error in the error dialog instead of only logging it; verify `bun run check`

## 4. Wrap-up

- [x] 4.1 `cd frontend && bun run test && bun run check && bun run lint`, `cargo test -p scotty --lib api::middleware`; verify all green
- [x] 4.2 Manual check against the local server: as a user with `manage` only on `client-a`, open a default-scope app; verify no Run/Rebuild buttons are shown, and that forcing the request (e.g. `curl` with the session cookie) returns 403 with the JSON message
- [x] 4.3 Add a kenkeep node under `frontend/` describing per-app permission resolution and inline action errors, run `npx kenkeep index rebuild`; create a bean, complete it with a summary, commit with `jj` as `fix(frontend): gate app actions per scope and show refused actions`
