## Why

Every app operation (create, run, stop, rebuild, destroy, purge, custom action) is tracked as a task that clients poll until it leaves `Running`. Since PR #897 only the state machine's completion handler terminates a task, so a handler that panics, or a top-level state machine whose error-state handler itself fails, leaves the task `Running` forever: `scottyctl` spins in its wait loop and the web UI shows a spinner indefinitely. Before #897 the same failures showed a wrong `Finished`; now they show nothing at all, which makes the gap worth closing.

## What Changes

- The task launcher for top-level state machines supervises the spawned state machine instead of dropping its join handle (existing TODO scotty-f1dd).
- If the state machine ends with an error or a panic while its task is still `Running`, the task is marked `Failed` with a status message naming the cause.
- No change for the happy path or for handler errors that already reach the error-state completion handler.

## Capabilities

### New Capabilities
- `task-lifecycle`: guarantees that every app-operation task reaches a terminal state (`Finished` or `Failed`) and defines what a client can rely on when it does.

### Modified Capabilities

## Impact

- `scotty/src/docker/helper.rs` (`run_sm`), possibly `scotty/src/state_machine.rs`.
- `scotty/src/docker/state_machine_handlers/context.rs` (`complete_task` reuse).
- New unit test with a panicking handler.
- No API or wire-format change; `TaskDetails` shape is unchanged.
