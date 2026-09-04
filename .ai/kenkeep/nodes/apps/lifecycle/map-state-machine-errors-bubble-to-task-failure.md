---
type: map
title: Task state is owned by a per-task actor; errors and panics always mark it Failed
description: >-
  One actor per task owns TaskDetails; the state machine Context holds the only
  TaskHandle, so a dropped, errored or panicked operation is always marked Failed
  and the terminal transition happens exactly once.
tags:
  - state-machine
  - tasks
  - docker
kk_schema_version: 3
kk_id: map-state-machine-errors-bubble-to-task-failure
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
Each task is owned by one actor (`scotty/src/tasks/actor.rs`): the actor holds the `TaskDetails`, folds `TaskEvent`s from a bounded mailbox, and publishes immutable `Arc<TaskDetails>` snapshots on a `watch` channel. `TaskManager` only stores snapshot receivers. Writers use a cloneable `TaskWriter` (output lines, step started, subprocess exited); the single non-clonable `TaskHandle` lives in the state machine `Context` and is the only thing that can terminate the task.

Guarantees that follow from ownership rather than discipline:
- `Terminate` is applied once; later terminations are logged and ignored, so the completion broadcast and status line happen exactly once.
- Dropping the `TaskHandle` without terminating (panic, swallowed error, forgotten completion handler) fails the task with "aborted unexpectedly", so nothing can stay `Running` forever. `run_sm` therefore drops the machine's JoinHandle.
- A subprocess exit (`run_task_and_wait`) only records the exit code; it never changes the task state. Only `TaskCompletionHandler` terminates, after refreshing app data (a failed refresh fails the task with `FailureKind::RefreshFailed`; a missing compose file after destroy is not a failure).
- Nested machines (`rebuild_app_prepare`/`purge_app_prepare` with `nested: true`) have no completion handler and no error state; they share the parent's context and let errors propagate via `helper::join_outcome`.
- Output streaming follows the snapshot `watch` and ends only when the snapshot is terminal and every line was sent, so the final status line is never lost.

New handlers write via `ctx.task.writer()` and must not terminate; if a new top-level operation is added it ends with `TaskCompletionHandler::success`/`failure`.
