## 1. Supervise the state machine

- [x] 1.1 In `scotty/src/docker/helper.rs` `run_sm`, replace `let _handle = sm.spawn(...)` with a spawned supervisor that awaits the join handle; verify `cargo clippy -p scotty --all-targets -- -D warnings` passes and `TODO(scotty-f1dd)` is removed or updated
- [x] 1.2 In the supervisor, when the join result is a panic or the machine returned `Err` and the task is still `Running`, log the cause with app name and task id and call `Context::complete_task(State::Failed, ...)` via `add_task_status_error`; verify by reading the task via `TaskManager::get_task_details` in the test from 2.1

## 2. Tests

- [x] 2.1 Add a unit test in `helper.rs` (or a `state_machine_handlers` test module) that runs `run_sm` with a state machine whose handler panics and asserts the task ends `Failed` with a finish time and a status line; verify with `cargo test -p scotty run_sm`
- [x] 2.2 Add a unit test where the error-state handler itself returns `Err` and assert the task still ends `Failed`; verify with `cargo test -p scotty run_sm`
- [x] 2.3 Add a unit test where the completion handler already set `Failed` before the machine returns `Err`, and assert the supervisor leaves state, finish time and output untouched; verify with `cargo test -p scotty run_sm`

## 3. Wrap-up

- [x] 3.1 Run `cargo test --workspace` and the ignored Docker test `cargo test --no-default-features --test test_scoped_create_visibility -- --ignored`; verify both pass
- [x] 3.2 Create a bean for this work, mark it completed with a summary, and commit with `jj` using a `fix(tasks):` conventional message referencing #894 as related
