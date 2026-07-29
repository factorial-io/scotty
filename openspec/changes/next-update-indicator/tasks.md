# Tasks: next-update-indicator

## 1. Server: track the next scheduled check

- [ ] 1.1 Add a next-app-check field to `AppState` (`scotty/src/app_state.rs`) as an `Arc<AtomicI64>` of unix seconds (`0` = unset), with a doc comment noting it is server-global
- [ ] 1.2 In `scotty/src/docker/setup.rs`, set it to sweep-start + `settings.scheduler.running_app_check` at the top of `schedule_app_check`, so both the startup call (line 15) and every scheduled run (line 42) record it
- [ ] 1.3 Add a small accessor on `AppState` returning `Option<DateTime<Local>>` (None when unset)

## 2. Server: expose it on app data

- [ ] 2.1 Add `next_check: Option<DateTime<Local>>` to `AppData` (`scotty-core/src/apps/app_data/data.rs`), documented as the next global sweep, defaulting to `None` in constructors
- [ ] 2.2 Stamp it in `inspect_app()` (`scotty/src/docker/find_apps.rs:146`) from the `AppState` accessor — read the global, do not compute `now + interval`
- [ ] 2.3 Fix up remaining `AppData` literals the compiler flags (`scotty/src/api/rest/handlers/apps/list.rs`, `scotty/src/api/secure_response_test.rs`, `scotty/src/docker/create_app.rs`) and the doctests in `data.rs`
- [ ] 2.4 Regenerate TypeScript bindings so `next_check` appears in `frontend/src/types.ts`

## 3. Frontend

- [ ] 3.1 Make `frontend/src/components/time-ago.svelte` sign-aware: drop the `Math.max(0, …)` clamp, render `in <n> <unit>` for future timestamps, keep `<n> <unit> ago` for past
- [ ] 3.2 Change the pill in `frontend/src/routes/dashboard/[slug]/+page.svelte:262` to `Next update <TimeAgo dateString={data.next_check} />`, rendered only when `next_check` is non-null

## 4. Verification

- [ ] 4.1 Unit test the sign-aware branch: one timestamp in the future and one in the past produce the forward and backward phrasings
- [ ] 4.2 `cargo test` and `cargo clippy` pass
- [ ] 4.3 `bun run check` and `bun run lint` pass in `frontend/`
- [ ] 4.4 Manual check with a short interval (`SCOTTY__SCHEDULER__RUNNING_APP_CHECK=1m`): pill counts down, and after rebuilding an app mid-interval the countdown does *not* reset to a full interval
