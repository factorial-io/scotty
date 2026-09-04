## Context

See proposal.md - Why. Today `TaskDetails` is an `Arc<RwLock<_>>` aliased by `TaskManager.processes` (scotty/src/tasks/manager.rs), `Context.task` (state_machine_handlers/context.rs), every handler, and the WebSocket output streamer (tasks/output_streaming.rs), which polls `get_task_output` and infers the end of a stream from `output_collection_active`. `start_process_with_settings` registers each subprocess under the task's own uuid, so the map entry for an operation is overwritten by every step and `last_exit_code` is a per-process fact stored on a per-operation record. `run_task_and_wait` polls the subprocess JoinHandle every 100 ms and rebroadcasts `TaskInfoUpdated`. Create nests the rebuild state machine and destroy nests purge on the same `Context`.

This slice keeps `StateMachine` and the seven lifecycle machines. It replaces who owns the task and how readers observe it.

## Goals / Non-Goals

**Goals:**
- One owner per task; terminal-once by construction, not by comment or latch.
- Readers subscribe; no polling of a lock, no ordering hazard between last line and terminal state.
- Hand-rolled actor (Ryhl, "Actors with Tokio") so the mechanics are visible: owned struct, mpsc mailbox, watch for snapshots, handle for producers.
- Wire compatibility: `TaskDetails`, `TaskOutputData`, `TaskInfoUpdated`, `TaskOutputStreamStarted/Ended` unchanged.

**Non-Goals:**
- Replacing `StateMachine` with linear async functions (later slice).
- Durability across server restart.
- A supervision tree or an actor library (`ractor`, `kameo`); revisit once the shape is proven.

## Decisions

**D1. Actor per task, spawned on task creation.**
```rust
enum TaskEvent {
    StepStarted { step: String },
    Output(OutputStreamType, String),
    SubprocessExited { exit_code: Option<i32> },
    Terminate(Outcome),
}
enum Outcome { Finished { status: String }, Failed { kind: FailureKind, status: String } }
enum FailureKind { StepFailed, RefreshFailed, Aborted }
struct TaskActor { details: TaskDetails, snapshot: watch::Sender<Arc<TaskDetails>>, mailbox: mpsc::Receiver<TaskEvent> }
```
The actor loop folds each event into `details`, then `snapshot.send_replace(Arc::new(details.clone()))`. `Terminate` is applied once: a second `Terminate` is logged and dropped. When the mailbox closes with the task still `Running`, the actor applies `Terminate { Failed, "aborted unexpectedly" }` itself. That is Erlang monitor semantics without a supervisor: the producers' handles are the link.
Alternative considered: keep the lock, add a latch (PR #897). Rejected: the latch is a runtime check of an invariant the type system can carry.

**D2. Two producer types.**
- `TaskWriter: Clone` sends `StepStarted`, `Output`, `SubprocessExited`. Handlers and the subprocess pump get this.
- `TaskHandle` (not `Clone`) wraps a `TaskWriter` plus the right to send `Terminate`. `impl Drop for TaskHandle`: if `terminate` was never called, send `Terminate { Failed, "aborted unexpectedly (internal error)" }`. Panics and cancellation unwind through `Drop`, so `run_sm` needs no supervisor task and `helper.rs` loses `join_outcome`.
`Context` holds the `TaskHandle` behind the existing `Arc<RwLock<Context>>`; `TaskCompletionHandler` calls `handle.terminate(...)`. Because `Terminate` is idempotent in the actor, the success handler failing and routing to the failure handler is harmless.
Alternative: make `terminate(self)` consume the handle. Rejected for this slice: `Context` is shared through `Arc<RwLock>` by the state machine framework, so consuming is not expressible until the control flow is rewritten.

**D3. Nested machines borrow, they do not own.** `create_app` and `destroy_app` spawn the nested machine over the same `Context` and therefore the same `TaskHandle`. Nested machines are built without a completion handler and without an error state (`nested: true`, as in PR #897); their errors propagate to the awaiting parent handler. Only the outer machine terminates and notifies. The actor rejects a second `Terminate` regardless, so a mistake here degrades to a log line, not a wrong state.

**D4. Subprocesses are steps, not tasks.** `TaskManager::start_process` takes a `TaskWriter`, spawns the child, pumps stdout/stderr as `Output` events, and returns `JoinHandle<anyhow::Result<i32>>`. A spawn error is returned (no `expect`) and also emitted as an `Output` line. `run_task_and_wait` awaits the handle instead of polling and fails the step when the exit code is not zero or is missing. `SubprocessExited` updates `last_exit_code` only. `TaskInfoUpdated` is broadcast by the actor on each snapshot rather than by the handler loop.

**D5. Readers use snapshots.** `TaskManager` becomes `HashMap<Uuid, TaskEntry { snapshot: watch::Receiver<Arc<TaskDetails>>, writer: TaskWriter }>`. `get_task_details` is `borrow().clone()`, lock-free. `output_streaming` subscribes with `watch::Receiver::changed()`, sends lines with sequence greater than the last sent, and ends the stream when the snapshot is terminal and all lines are sent. Since a terminal snapshot is produced after the final `Output` event was folded, the stream cannot end early. `output_collection_active` stays in `TaskDetails` for the wire but is derived (`state == Running`).
Alternative: `broadcast` channel for lines. Rejected: lagging receivers lose lines; `watch` plus sequence numbers gives at-least-once without a second buffer.

**D6. Metric and finish time move into the actor** (`Terminate` fold). `record_task_finished` fires once per task.

**D7. Failure reason is structured internally, prose on the wire.** `Terminate` carries an `Outcome` whose failure variant names a closed `FailureKind`; the actor renders the status line from it. `TaskDetails` keeps only `state` plus the status line, so nothing on the wire changes and no client has to branch on prose. When a consumer needs the kind (retry policy, a metric label, a frontend badge), adding `reason: Option<FailureKind>` to `TaskDetails` with `#[serde(default)]` is one field, and the data already exists at every failure site. Prior art: Kubernetes `Condition.reason` (program) next to `message` (human); Erlang exit reasons are terms, not strings. Alternative: string only. Rejected because reconstructing the kind from prose later is guesswork, and the enum costs about ten lines now.

**D8. Cleanup.** `run_cleanup_task` removes entries whose snapshot is terminal and older than the TTL; dropping the entry drops the last `watch::Receiver`, which is fine because the actor exits when its mailbox closes.

## Risks / Trade-offs

- [Mailbox backpressure on chatty subprocesses] → bounded mpsc (e.g. 1024); the output pump `await`s `send`, which slows the reader of the child pipe, which applies OS backpressure to the child. Acceptable; log if the queue stays full.
- [Snapshot cloning cost per event with large outputs] → snapshots are `Arc<TaskDetails>`; the actor clones `details` once per event. Batch `Output` events through the existing `TimedBuffer` in the pump (10 lines or 100 ms) so a build log produces tens, not thousands, of snapshots per second.
- [`Drop` on `TaskHandle` runs inside a panicking unwind] → the send is `try_send` on an unbounded-enough mailbox and never panics; if the mailbox is full the actor's close-detection path fails the task anyway.
- [Frontend relies on `output_collection_active`] → keep the field, derive it; no frontend change.
- [Nested flag still exists] → accepted for this slice; disappears when control flow becomes linear.

## Migration Plan

Each step compiles and ships alone:
1. Add `tasks/actor.rs` with `TaskActor`, `TaskEvent`, `TaskWriter`, `TaskHandle`, snapshot type, unit tests. Nothing uses it yet.
2. `TaskManager` spawns an actor per task and stores `TaskEntry`; `get_task_details`/`get_task_list`/`add_task_*` route through it. Readers unchanged in behavior.
3. `Context` carries `TaskHandle`; `TaskCompletionHandler` terminates through it; `run_sm` stops dropping the JoinHandle. Delete `complete_task`'s lock dance.
4. Subprocess identity: `start_process` takes a `TaskWriter`, returns a JoinHandle of the exit code; `run_task_and_wait` awaits.
5. `output_streaming` and the WebSocket handler subscribe to the `watch`.
6. Nested mode for rebuild/purge; scottyctl trusts `state`; Docker regression test for create and destroy.
Rollback at any step is a revert of that step.
