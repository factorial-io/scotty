# Proposal: next-update-indicator

## Why

The app detail page ends with a pill reading `Updated <n> minutes ago`, built from `AppData.last_checked` (`frontend/src/routes/dashboard/[slug]/+page.svelte:262`). Backward-looking staleness is the less useful half of the information: what a user wants to know when an app's status looks wrong is *when Scotty will look again*, so they can decide whether to wait or trigger an action themselves.

The frontend cannot derive that on its own. It does not know `scheduler.running_app_check`, and even if it did, `last_checked + interval` would be wrong: `inspect_app()` is also called from the state machine (`update_app_data_handler.rs:31`, `task_completion_handler.rs:86`), so any manual run/stop/rebuild bumps `last_checked` out of band from the scheduler's sweep. The extrapolation is off by up to a full interval precisely when the user is most likely watching the page — right after clicking Rebuild.

So the server must tell the frontend when the next sweep is due.

## What Changes

- The server tracks the next scheduled app-check time (a single global value, since `clokwerk` runs one sweep for all apps) in `AppState`, written by the sweep itself.
- `AppData` gains a `next_check` timestamp, stamped in `inspect_app()` alongside `last_checked` by reading that global value — not by extrapolating from "now".
- The app detail pill switches from `Updated <n> minutes ago` to `Next update in <n> minutes`, driven by `next_check`.
- `time-ago.svelte` becomes sign-aware: future timestamps render as `in <n> <unit>`, past timestamps keep rendering `<n> <unit> ago`. Existing call sites (`tasks-table`, `last-started`, task detail) pass past timestamps and are unaffected.
- When `next_check` is null (app created but never swept), the pill is not rendered.

`last_checked` stays on `AppData` — the app-list metrics collector uses it (`scotty/src/metrics/app_list.rs:26`) and it remains the correct source for staleness.

## Capabilities

### New Capabilities

- `app-refresh-indicator`: What the app detail page tells the user about Scotty's app-state refresh cycle, and how the next-refresh time is derived and exposed.

### Modified Capabilities

<!-- none -->

## Impact

- `scotty/src/app_state.rs` — new field holding the next scheduled check time.
- `scotty/src/docker/setup.rs` — the sweep (and the startup check) records the next due time.
- `scotty/src/docker/find_apps.rs` — `inspect_app()` stamps `next_check`.
- `scotty-core/src/apps/app_data/data.rs` — new `next_check` field; ts-rs regenerates `frontend/src/types.ts`.
- Every `AppData` literal in tests/handlers that names `last_checked` needs `next_check` too (`scotty/src/api/rest/handlers/apps/list.rs`, `scotty/src/api/secure_response_test.rs`, `scotty/src/docker/create_app.rs`).
- `frontend/src/components/time-ago.svelte`, `frontend/src/routes/dashboard/[slug]/+page.svelte`.
- No new endpoints, no scottyctl changes, no config changes.
