## Context

When an app is stopped, Traefik has no route for its domain and a lowest-priority catch-all router forwards the request to Scotty. `landing_or_frontend_handler` (`scotty/src/api/rest/handlers/landing.rs`) looks up the app by Host header and, for `Stopped` apps, redirects to the SvelteKit landing page, which requires a Scotty login and `manage` permission to start the app.

Facts that shape the design:

- `AppSettings.basic_auth` stores the plaintext `(user, password)`. It is only hashed when Traefik labels are written (`docker/loadbalancer/traefik.rs`), so Scotty can verify a `Basic` header itself.
- Scotty sets no `realm` on the Traefik `basicauth` middleware, so Traefik uses its default realm `traefik`.
- App status comes from container state (`get_app_status_from_services`). Between spawning `run_app` and the first container appearing, the app still reads as `Stopped`.
- `run_operation` (`docker/helper.rs`) takes an optional notification and already runs the operation in a background task.
- `subtle` is already a dependency and is used for constant-time token comparison in `auth_core.rs`.

Prior art: Sablier's "dynamic" strategy (a Traefik plugin that serves a self-refreshing waiting page while it starts containers), Knative's activator (holds requests while it scales from zero), and Heroku's idle-dyno wake.

## Goals / Non-Goals

**Goals:**
- Wake a stopped app when someone holding its basic-auth credentials requests its domain.
- Never start an app for unauthenticated traffic (bots, crawlers, uptime checks).
- Exactly one start per app, however many concurrent requests arrive.
- No frontend changes and no new public API endpoint.

**Non-Goals:**
- Holding or proxying the original request until the app is up (Knative-style). The browser refreshes instead.
- Showing start progress or task output to the waking user.
- Autowake for apps without basic auth, or with any other credential type.
- Scale-to-zero on idle. Apps still stop through their TTL as today.

## Decisions

### Wake in the catch-all handler, not through the landing page
The handler starts the app and returns a `503` HTML page with `<meta http-equiv="refresh" content="5">`, `Retry-After: 5` and the existing no-cache headers. The browser refreshes the original URL, and once Traefik has the app's route the request never reaches Scotty again.

*Alternative:* a public wake endpoint plus public status polling from the SvelteKit landing page. Rejected because it needs new anonymous API surface, frontend work, and `return_url` validation without authenticated app info.

### Basic auth is the only gate, checked by Scotty
Branch order for a `Stopped` app:
1. `autowake` is false, or `basic_auth` is `None` → existing landing redirect, unchanged.
2. The `Authorization: Basic` header is missing, malformed, or the user or password does not match (constant-time comparison via `subtle`) → `401` with `WWW-Authenticate: Basic realm="traefik"`.
3. Valid → wake (subject to the in-flight guard) and return the starting page.

Using Traefik's realm means the browser treats Scotty and the running app as the same protection space and resends cached credentials, so the user sees a single prompt.

*Alternative:* a separate per-app wake token. Rejected because it is one more secret to distribute, and basic auth already keeps bots out.

### In-flight guard on AppState
Add `waking: Arc<Mutex<HashSet<String>>>` (std mutex, never held across `.await`) to `AppState`. The handler inserts the app name and only spawns `wake_app` if the insert succeeded. The name is removed when the wake operation finishes, whether it succeeds or fails. A drop guard inside the spawned future ensures it is also removed on panic.

*Alternative:* look in the task manager for a running task on the app. Rejected because tasks are not indexed by app and that check also races. The set is atomic and small.

Ceiling: the set is per process. That is fine, because Scotty runs as a single instance.

### `wake_app` without notification
Add `wake_app` to `docker/run_app.rs`: it runs the same `run_steps` through `run_operation` but passes `notification = None`. The task still exists (for logs and tracing), but nobody subscribes to its output.

### Any HTTP method wakes
Basic auth already filters out bots, and restricting by method buys nothing. Request bodies are not forwarded: a non-GET request gets the same `503` page.

### Validation at create time plus a runtime fallback
`create_app_handler` rejects `autowake: true` with `basic_auth: None` (`400`, clear message). Settings can still end up in that state through adoption or manual edits, so the handler treats it as "autowake off" (decision 1 above) rather than waking without a check.

## Risks / Trade-offs

- [Anyone with the basic-auth credentials can start the app] → Intended. These credentials already grant access to the running app. Admins opt in per app.
- [Scotty now answers basic-auth guesses for stopped apps] → It is the same oracle Traefik already provides for running apps. Constant-time comparison is used, and the catch-all route can sit behind the existing rate limiting.
- [A failed start retries on every refresh while the user stays on the page] → The guard prevents parallel starts but not sequential ones. Accepted for now; add a cooldown after a failed wake if it shows up in practice.
- [The browser may not reuse credentials if a custom middleware changes the realm] → Only the built-in basic-auth middleware is supported. Document that `autowake` relies on it.
- [Status stays `Stopped` until containers appear, so the 503 page shows a generic "starting" message] → Acceptable, since no progress is shown by design.

## Migration Plan

`autowake` defaults to `false` through serde, so existing apps and stored settings are unaffected. Rollback means dropping the field: unknown fields in stored settings are ignored only if the struct permits it, so check that `AppSettings` does not use `deny_unknown_fields` (it does not today).

## Open Questions

- Should `autowake` also be settable as a blueprint default? Not in this change.
