## Context

See proposal.md - Why. Top-level state machines are started by `run_sm` in `scotty/src/docker/helper.rs`, which spawns the machine and drops the `JoinHandle` so the HTTP handler can return immediately. `StateMachine::spawn` returns `JoinHandle<anyhow::Result<()>>`; a handler `Err` is routed to the configured error state whose `TaskCompletionHandler::failure` calls `Context::complete_task`. Two paths bypass that: a panic in any handler (tokio catches it, the join result is `Err(JoinError)`), and an `Err` returned by the error-state handler itself, which `StateMachine::run` discards. Nested machines (create wraps rebuild, destroy wraps purge) are already awaited by their parent handler and turn a panic into a parent error, so they are covered once the top level is.

## Goals / Non-Goals

**Goals:**
- Every task started through `run_sm` ends `Finished` or `Failed`.
- Keep `Context::complete_task` the single place that writes a terminal state.
- Do not delay the HTTP response.

**Non-Goals:**
- Recovering or retrying a panicked operation.
- Making `StateMachine` itself aware of tasks; it stays generic over its context.
- Client-side timeouts in `scottyctl` or the frontend.

## Decisions

**Supervise in `run_sm`, not in `StateMachine::spawn`.** `run_sm` is the only place that both owns the task and knows the concrete `Context`. `spawn` is generic over `C` and used by nested machines that are awaited by a parent. Alternative considered: a `Drop` guard inside `Context` that fails the task if it is still `Running`; rejected because `Context` is held in an `Arc` shared with output streaming, so the drop point is not tied to the state machine ending.

**Wrap instead of await.** `run_sm` spawns a small supervisor task that awaits the state machine's join handle and then, if the task is still `Running`, calls `complete_task(State::Failed, ...)` with the join error or the machine's error. The API handler still returns as soon as the machine is spawned.

**Never overwrite a terminal state.** The supervisor reads the task state under the lock and only acts when it is `Running`, so the normal completion handler's message and timestamps win.

**Message content.** For a panic: "Operation for app '<name>' aborted unexpectedly (internal error)". For a swallowed error-handler failure: the error chain. Both are added through `add_task_status_error` so they appear in the task output clients already display.

## Risks / Trade-offs

- [Supervisor itself panics] → It only does a lock, a compare and a `complete_task` call; keep it free of `unwrap` on external data.
- [Double completion race between the completion handler and the supervisor] → Both go through the same `RwLock<TaskDetails>`; the supervisor checks and writes under one write lock via `complete_task`, and the handler always runs before the join handle resolves, so ordering is deterministic.
- [Panic output lost] → tokio's `JoinError` carries the panic payload; log it with `error!` including app name and task id before failing the task.

## Migration Plan

No data or config migration. Deploy as a normal patch release; rollback is a revert.
