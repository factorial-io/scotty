## 1. Foundation

- [x] 1.1 Add `run_operation` to `scotty/src/docker/helper.rs` per design D2 (creates `Context` as `Arc<Context>`, starting status line, spawn, refresh, single `terminate`, notification on success); verify unit tests: success ends `Finished` with the success line, a step error ends `Failed` with the error text in the status line, a refresh failure ends `Failed` and suppresses the notification, a panicking closure ends `Failed` with "aborted unexpectedly"
- [x] 1.2 Rename `docker/state_machine_handlers/` to `docker/steps/`, move `context.rs`, drop `RwLock` from `Context` usage; verify `cargo build -p scotty` compiles with the old machines still calling into it (done in one pass together with the operations; no shims were needed)

## 2. Step functions

- [x] 2.1 Convert `CreateDirectoryHandler`, `RemoveDirectoryHandler`, `SaveSettingsHandler`, `SaveFilesHandler`, `UpdateAppDataHandler` into `async fn` steps taking `&Context` plus their former fields; verify `cargo test -p scotty` passes
- [x] 2.2 Convert `RunDockerLoginHandler`, `RunDockerComposeHandler`, `WaitForAllContainersHandler`, `EnsureAppNetworkHandler`, `TeardownAppNetworkHandler` into steps; each wraps its error with `.context("<step name>")`; verify existing network handler unit tests pass against the new functions
- [x] 2.3 Convert `CreateLoadBalancerConfig` and `RunPostActionsHandler` into steps; verify their existing unit tests pass

## 3. Operations (one commit each)

- [x] 3.1 `stop_app` and `force_stop_app` via `run_operation` + `stop_steps`; covered by `run_operation` unit tests (status line order) and the Docker regression test; no per-operation mock test since steps shell out to docker-compose
- [x] 3.2 `purge_app` via `purge_steps(ctx, method)`; `PurgeAppMethod::compose_args` holds the two argument lists; verified by review and the Docker test's destroy phase
- [x] 3.3 `run_app` via `run_steps`; step order is the function body (login, network, up, wait, post actions, update); verified by review
- [x] 3.4 `run_app_custom_action` via `custom_action_steps`; `validate_action_exists` kept unchanged; the `AppCustomActionCompleted` notification carries the action; verified by `cargo test -p scotty`
- [x] 3.5 `rebuild_app` via `rebuild_steps(ctx, recreate_load_balancer_config)`; the `recreate` branch runs only when `recreate && app.settings.is_some()` (single `if let` at the top of `rebuild_steps`); verified by review
- [x] 3.6 `create_app` via `create_steps` calling `rebuild_steps(ctx, false)`; remove `RunDockerComposeBuildHandler`; verify `cargo test --no-default-features --test test_scoped_create_visibility -- --ignored` passes with exactly one completion line
- [x] 3.7 `destroy_app` via `destroy_steps` calling `purge_steps(ctx, Down)`; remove `RunDockerComposeDownHandler`; verify the same Docker test's destroy phase ends `Finished`

## 4. Removal

- [x] 4.1 Delete `scotty/src/state_machine.rs`, `task_completion_handler.rs`, `run_sm`, `join_outcome`, the seven state enums and the `nested` parameters; verify `rg -n "StateMachine|StateHandler|nested" scotty/src` returns nothing and `cargo clippy --workspace --all-targets -- -D warnings` is clean
- [x] 4.2 Move the `failed_refresh_still_terminates_task` and `missing_compose_file_completes_successfully` tests onto `run_operation`; verify `cargo test -p scotty --lib docker::helper` passes

## 5. Wrap-up

- [x] 5.1 `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --no-default-features`, Docker regression test; verify all green
- [x] 5.2 Update kenkeep node `map-state-machine-errors-bubble-to-task-failure` (rename to describe `run_operation` + linear steps, drop state machine wording) and the `apps/lifecycle` index; run `npx kenkeep index rebuild`; verify no stale-node warning
- [x] 5.3 Create a bean, complete it with a summary, and commit as one change with `jj` (user asked for a single commit on a new branch) `refactor(docker): replace lifecycle state machines with linear operations`; verify `jj log` shows it on `refactor/linear-operations` on top of `refactor/task-actor`
