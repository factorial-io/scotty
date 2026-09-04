---
# scotty-5rhr
title: Fail the task when a state machine panics or its error handler fails
status: completed
type: bug
priority: normal
created_at: 2026-09-04T21:54:19Z
updated_at: 2026-09-04T21:54:41Z
---

Follow-up to #894 / PR #897 and TODO scotty-f1dd. run_sm dropped the state machine JoinHandle, so a handler panic, or an error-state handler that itself failed, left the task Running forever. OpenSpec change: openspec/changes/fail-task-on-state-machine-panic.

- [x] Supervise the spawned state machine in run_sm
- [x] Fail the task (once, without overwriting a terminal state) on panic or swallowed error
- [x] Unit tests: panic, failing error handler, no overwrite
- [x] Workspace tests, clippy and Docker regression test pass

## Summary of Changes

- scotty/src/docker/helper.rs: run_sm now spawns a supervisor that awaits the state machine JoinHandle. If it ended with Err or a panic while the task is still Running, the task is failed via Context::complete_task with a status line naming the cause; an already terminal state is never overwritten. The scotty-f1dd TODO is resolved.
- Three unit tests in helper.rs cover panic, failing error-state handler, and no-overwrite.
- OpenSpec change fail-task-on-state-machine-panic holds proposal/spec/design/tasks (all ticked).
