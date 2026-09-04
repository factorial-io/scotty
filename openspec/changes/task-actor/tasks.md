## 1. Actor core

- [x] 1.1 Create `scotty/src/tasks/actor.rs` with `TaskEvent`, `Outcome`/`FailureKind`, `TaskActor` (owns `TaskDetails`, folds events, publishes `watch<Arc<TaskDetails>>`), `TaskWriter` (Clone), `TaskHandle` (not Clone, `terminate`, `Drop` fails an unterminated task); verify unit tests: terminal-once (second `Terminate` ignored), drop-without-terminate yields `Failed` with "aborted unexpectedly", output line order preserved, terminal snapshot contains the final status line
- [x] 1.2 Move finish time and `record_task_finished` into the `Terminate` fold; verify a unit test asserts the metric recorder is called once for two `Terminate` events

## 2. Registry

- [x] 2.1 Change `TaskManager` to spawn one actor per task and store the snapshot receiver per task (no writer: holding one would keep the actor alive after its owner is gone); `get_task_details`, `get_task_list` read snapshots, the `add_task_*` helpers are gone and callers use `TaskWriter`; verify `cargo test -p scotty` passes and REST `GET task/{id}` returns the same JSON shape (existing `secure_response_test`)
- [x] 2.2 Rewrite `run_cleanup_task` to remove terminal entries older than the TTL; verify a unit test with a short TTL removes a terminated task and keeps a running one

## 3. Ownership in the state machines

- [x] 3.1 `Context` holds a `TaskHandle`; `Context::complete_task` is removed, callers call `task.terminate(Outcome)` (the status line is part of the terminate fold); verify existing state machine unit tests pass
- [x] 3.2 `run_sm` creates the actor, moves the `TaskHandle` into `Context`, and drops the machine's JoinHandle (the `TaskHandle` in the context fails the task on panic, so no supervisor is needed); verify a unit test with a panicking handler ends `Failed`, and one where the error-state handler itself fails ends `Failed`
- [x] 3.3 `TaskCompletionHandler` terminates through the handle; a failed app-data refresh fails the task and propagates; a missing compose file (destroy) skips the refresh; verify unit tests `failed_refresh_still_terminates_task` and `missing_compose_file_completes_successfully` (ported from PR #897)

## 4. Subprocesses as steps

- [x] 4.1 `TaskManager::start_process` takes a `TaskWriter`, returns `JoinHandle<anyhow::Result<i32>>`, emits `StepStarted`/`Output`/`SubprocessExited`, returns spawn errors instead of panicking and records them as output; verify unit tests: `true` leaves the task `Running` with exit code 0, `false` records exit code 1, an unknown command records the error text and `None`
- [x] 4.2 `run_task_and_wait` awaits the JoinHandle, fails the step on non-zero or missing exit code, and stops broadcasting `TaskInfoUpdated` itself (the actor broadcasts on `StepStarted`/`SubprocessExited`/`Terminate`; covered by the Docker regression test, no dedicated messenger unit test)

## 5. Read path

- [x] 5.1 `output_streaming` subscribes to the task's `watch`, sends lines by sequence number, ends the stream only when the snapshot is terminal and all lines were sent; verify a unit test that terminates a task with 3 pending lines delivers all 3 before `TaskOutputStreamEnded`
- [x] 5.2 WebSocket and REST task handlers read snapshots; `output_collection_active` is derived from `state`; verify `cargo test -p scotty` and `cd frontend && bun run check` pass unchanged

## 6. Nesting, client, regression test

- [x] 6.1 `rebuild_app_prepare` and `purge_app_prepare` take `nested: bool` (no completion handler, no error state); create and destroy pass `true`; verified by the Docker regression test (create ends with exactly one completion line); `StateMachine` handlers are private so no direct unit test
- [x] 6.2 scottyctl: both wait loops fail on `state == Failed` regardless of exit code and show the exit code only when non-zero; verify `cargo test -p scottyctl`
- [x] 6.3 Port `scotty/tests/test_scoped_create_visibility.rs` from PR #897 (create with scoped token, `apps/info` 200, exactly one completion line, destroy ends `Finished`); verify `cargo test --no-default-features --test test_scoped_create_visibility -- --ignored` passes with Docker

## 7. Wrap-up

- [x] 7.1 `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, Docker test; verify all green
- [x] 7.2 Update the kenkeep node `map-state-machine-errors-bubble-to-task-failure` to describe the actor ownership; verify `npx kenkeep index rebuild` reports no stale nodes
- [x] 7.3 Create a bean, complete it with a summary, and commit with `jj` as `refactor(tasks): own task state in a per-task actor` referencing #894; verify `jj log` shows one commit on top of main
