## 1. Backend: capture exit code

- [x] 1.1 Add `exit_code: Option<i64>` to `ContainerState` in `scotty-core/src/apps/app_data/container.rs`
- [x] 1.2 Populate `exit_code` from Bollard `state.exit_code` in `inspect_docker_container` (`scotty/src/docker/find_apps.rs`)
- [x] 1.3 Update any other `ContainerState` constructors / test fixtures to set `exit_code` (default `None`)

## 2. Backend: classification helpers

- [x] 2.1 Add `ContainerState::is_completed()` (status `Exited` && `exit_code == Some(0)`) in `container.rs`; keep `is_running()` unchanged
- [x] 2.2 Add unit tests for `is_running` / `is_completed` covering Running, Created, Restarting, Exited(0), Exited(non-zero), Dead

## 3. Backend: app status aggregation

- [x] 3.1 Rework `get_app_status_from_services` in `scotty-core/src/apps/app_data/status.rs` to use running + completed counts per design (Running when running>0 and running+completed==total; Stopped when none running and none completed; Starting otherwise). NOTE: keeps strict `Running` count (not `is_running()`, which also counts Created/Restarting) to preserve original "Created => Starting" semantics.
- [x] 3.2 Update/add unit tests in `status.rs`: running web + completed init => Running; partially started => Starting; all exited => Stopped; failed container => Starting
- [x] 3.3 Verify `AppData::new` (`data.rs`) and `urls()` still compile with the changes; no app-level status gating added to `urls()`

## 4. Type bindings

- [x] 4.1 Regenerate ts-rs bindings (ts-generator) so `ContainerState` carries `exit_code`. NOTE: N/A — `ContainerState`/`AppData` have no ts-rs `#[derive(TS)]` and are not produced by ts-generator; the frontend type is hand-maintained in `types.ts`. `exit_code` reaches the frontend via serde serialization of `Vec<ContainerState>`.
- [x] 4.2 Add `status` (confirm present) and `exit_code?: number` to the `AppService` interface in `frontend/src/types.ts`

## 5. Frontend: per-service URL gating

- [x] 5.1 In `frontend/src/components/app-service-button.svelte`, gate on `service.status !== 'Running'` instead of the passed-in app `status`; removed the now-unused `status` prop
- [x] 5.2 Update caller `routes/dashboard/+page.svelte` to no longer rely on app-level `status` for URL gating
- [x] 5.3 Update caller `routes/dashboard/[slug]/+page.svelte`
- [x] 5.4 Update caller `routes/dashboard/[slug]/[service]/+page.svelte`

## 6. Verification

- [x] 6.1 `cargo test` (scotty-core + scotty) passes, including new/updated status tests (538 passed, 20 ignored)
- [x] 6.2 `cd frontend && bun run check && bun run lint` pass (0 errors; pre-existing unrelated `node` type-defs warning)
- [ ] 6.3 Manual: app with a running web service + an `Exited 0` init container shows the web URL and reports app status `Running` (requires live Docker env; covered by unit tests `running_web_with_completed_init_is_running`)
- [ ] 6.4 Manual: app with a crashed (`Exited` non-zero) container shows that service disabled, healthy services still show URLs (requires live Docker env; covered by `failed_container_with_running_sibling_is_starting` + per-service frontend gating)
