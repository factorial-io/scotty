# Tasks: next-update-indicator

## 1. Server: track the next scheduled check

- [x] 1.1 Add a next-app-check field to `AppState` (`scotty/src/app_state.rs`) as an `Arc<AtomicI64>` of unix seconds (`0` = unset), with a doc comment noting it is server-global
- [x] 1.2 In `scotty/src/docker/setup.rs`, set it to sweep-start + `settings.scheduler.running_app_check` at the top of `schedule_app_check`, so both the startup call (line 15) and every scheduled run (line 42) record it
- [x] 1.3 Add a small accessor on `AppState` returning `Option<DateTime<Local>>` (None when unset)

## 2. Server: expose it on app data

- [x] 2.1 Add `next_check: Option<DateTime<Local>>` to `AppData` (`scotty-core/src/apps/app_data/data.rs`), documented as the next global sweep, defaulting to `None` in constructors
- [x] 2.2 Stamp it in `inspect_app()` (`scotty/src/docker/find_apps.rs:146`) from the `AppState` accessor — read the global, do not compute `now + interval`
- [x] 2.3 Fix up remaining `AppData` literals the compiler flags (`scotty/src/api/rest/handlers/apps/list.rs`, `scotty/src/api/secure_response_test.rs`, `scotty/src/docker/create_app.rs`) and the doctests in `data.rs`
- [x] 2.4 Add `next_check` to the `App` interface in `frontend/src/types.ts` — hand-maintained, not ts-rs generated (`ts-generator` only covers the websocket/task types under `frontend/src/generated/`)

## 3. Frontend

- [x] 3.1 Make `frontend/src/components/time-ago.svelte` sign-aware: drop the `Math.max(0, …)` clamp, render `in <n> <unit>` for future timestamps, keep `<n> <unit> ago` for past
- [x] 3.2 Change the pill in `frontend/src/routes/dashboard/[slug]/+page.svelte:262` to `Next update <TimeAgo dateString={data.next_check} />`, rendered only when `next_check` is non-null

## 4. Verification

- [x] 4.1 Check the sign-aware branch in both directions. The frontend has no test runner; rather than add one for a single pure function, the logic was extracted to `frontend/src/lib/relative-time.ts` with a framework-free self-check in `relative-time.check.ts` (`bun src/lib/relative-time.check.ts`). `bun:test` was tried first but svelte-check cannot resolve it without adding `@types/bun`.
- [x] 4.2 `cargo test` and `cargo clippy` pass
- [x] 4.3 `bun run check` and `bun run lint` pass in `frontend/`
- [x] 4.4 Verified against a live server with `SCOTTY__SCHEDULER__RUNNING_APP_CHECK=1m`: `next_check` came back exactly one interval after `last_checked`, and after a mid-interval `app:run` on `simple-nginx-2` (`last_checked` 23:12:00 -> 23:12:25) `next_check` stayed at the real next sweep (23:13:00) instead of drifting to 23:13:25. Server log clean. The rendered pill itself was not eyeballed in a browser -- it is a template swap covered by `svelte-check` plus the `formatRelativeTime` self-check.
