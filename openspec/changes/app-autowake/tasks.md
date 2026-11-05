## 1. Setting

- [ ] 1.1 Add `#[serde(default)] pub autowake: bool` to `AppSettings` (`scotty-core/src/apps/app_data/settings.rs`), including `Default`, and fix struct literals in tests
- [ ] 1.2 Regenerate TypeScript bindings via ts-generator and make sure `bun run check` passes

## 2. Server

- [ ] 2.1 Reject `autowake: true` without `basic_auth` in `create_app_handler` with a 400 error, plus a test
- [ ] 2.2 Add `wake_app` in `scotty/src/docker/run_app.rs`: `run_steps` via `run_operation` with `notification = None`
- [ ] 2.3 Add a `waking: Arc<Mutex<HashSet<String>>>` in-flight set to `AppState` and initialize it everywhere `AppState` is built (including test helpers)
- [ ] 2.4 Add a helper that parses `Authorization: Basic` and compares user and password with `subtle::ConstantTimeEq`, with unit tests for valid, wrong, malformed and missing headers
- [ ] 2.5 Add the autowake branch to `landing_or_frontend_handler` for `Stopped` apps: fall back to the redirect when autowake is off or basic_auth is missing; return 401 with `WWW-Authenticate: Basic realm="traefik"` on bad credentials; on valid credentials insert into `waking` and spawn `wake_app` (removing the name when the operation ends, on panic too); return the 503 self-refreshing page with `Retry-After: 5`
- [ ] 2.6 Integration tests (axum-test) for the spec scenarios: no header, wrong credentials, valid credentials (503 and a start is triggered), concurrent requests trigger one start, autowake without basic_auth redirects

## 3. CLI

- [ ] 3.1 Add the `--autowake` flag to `app:create` in `scottyctl/src/cli.rs` and pass it into `AppSettings` in `commands/apps/management.rs`

## 4. Docs

- [ ] 4.1 Document `autowake` (basic auth required, realm reuse, no notification) in the user docs and add a kenkeep node next to `map-default-backend-landing-page`
