## Why

Stopped apps can already be started from the landing page, but only by a Scotty user with `manage` permission who logs in first. For apps shared with external reviewers (clients, QA) that is a dead end: they hit a stopped app and cannot wake it. Apps protected by basic auth already have a credential those reviewers hold, so that credential can safely authorize a wake without a Scotty login, while still keeping bots and crawlers from starting apps.

## What Changes

- New per-app setting `autowake: bool` (default `false`) in `AppSettings`.
- The Traefik catch-all handler (`landing.rs`) wakes a stopped app on request when `autowake` is set and the app has `basic_auth`:
  - Missing or wrong credentials → `401` with `WWW-Authenticate: Basic realm="traefik"` (Traefik's default realm, so the browser reuses the credentials once the app's own route is live).
  - Valid credentials → start the app in the background (no Scotty login, no task output shown, no `AppStarted` notification) and answer with a self-refreshing `503` "Starting" page.
- An in-memory in-flight guard ensures concurrent requests trigger only one start per app.
- App creation rejects `autowake: true` without `basic_auth`. At runtime, an `autowake` app without `basic_auth` falls back to the existing landing-page redirect.
- `scottyctl app:create --autowake` flag.

## Capabilities

### New Capabilities
- `app-autowake`: waking stopped apps on an incoming request to their domain, gated by the app's basic-auth credentials.

### Modified Capabilities

None.

## Impact

- `scotty-core/src/apps/app_data/settings.rs`: new `autowake` field (serde default, TypeScript type regenerated via ts-rs).
- `scotty/src/api/rest/handlers/landing.rs`: new autowake branch for stopped apps.
- `scotty/src/app_state.rs`: in-flight set of waking apps.
- `scotty/src/docker/run_app.rs`: a wake variant of `run_app` without notification.
- `scotty/src/api/rest/handlers/apps/create.rs`: validation rule.
- `scottyctl/src/cli.rs`, `scottyctl/src/commands/apps/management.rs`: `--autowake` flag.
- Security: anyone holding the app's basic-auth credentials can start it; Scotty answers the same credential check Traefik answers for running apps, so no new secret is exposed.
