## Why

App-operation tasks are tracked in an `Arc<RwLock<TaskDetails>>` that is written from several places: the task manager after every subprocess, the completion handler, and every reader that pokes at it. Because no single component owns the task, the task manager marked a task `Finished` after the first `docker compose` subprocess exited (issue #894, a scoped token got 403 on `apps/info` because Casbin scopes were not yet synced), and the attempt to fix that in PR #897 uncovered eight further latent defects, each a consequence of shared, multi-writer state. This change gives every task exactly one owner.

## What Changes

- Introduce a per-task actor that owns `TaskDetails` by value. Producers send events (step started, output line, subprocess exited, terminal) over a channel; the actor folds them into the task and publishes snapshots on a `watch` channel.
- Introduce a non-clonable `TaskHandle` that is the only way to emit the terminal event. Dropping it without a terminal event (panic, cancellation, swallowed error) fails the task. Nested state machines borrow the handle instead of sharing the task.
- Subprocesses get their own identity; a subprocess exit updates `last_exit_code` and emits a step event but never changes the task state.
- REST, WebSocket and output streaming read from the actor's snapshot and subscribe to its `watch` instead of polling a lock. Terminal state and the final status line are published in one snapshot.
- The state machine framework and the seven lifecycle machines keep driving the steps. Control flow is out of scope for this slice.
- Carried over from PR #897 as design-independent: scottyctl trusts task `state` over `last_exit_code`; a subprocess that cannot be spawned records the error and fails the step; the Docker-backed create-and-destroy regression test.
- **BREAKING (internal only)**: `TaskManager::start_process` and the `add_task_*` helpers change signature. The wire format of `TaskDetails` and all WebSocket task messages is unchanged.

## Capabilities

### New Capabilities
- `task-lifecycle`: guarantees of app-operation tasks: every task reaches exactly one terminal state, only the operation owner sets it, subprocesses never terminate a task, and observers never see a terminal task with truncated output.

### Modified Capabilities

## Impact

- `scotty/src/tasks/`: new `actor.rs` (actor, events, handle, snapshot), `manager.rs` becomes a registry of actors, `output_streaming.rs` subscribes instead of polling.
- `scotty/src/docker/helper.rs`, `state_machine_handlers/context.rs`, `task_completion_handler.rs`, `run_task_and_wait.rs`: use `TaskHandle`.
- `scotty/src/api/rest/handlers/tasks.rs`, `scotty/src/api/websocket/handlers/tasks.rs`: read snapshots.
- `scottyctl/src/api.rs`: failure decided by `state`.
- Metrics: `record_task_finished` fires once per task, inside the actor.
- No change to `scotty-types::TaskDetails`, `WebSocketMessage`, or the frontend.
