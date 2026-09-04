---
# scotty-676o
title: Own task state in a per-task actor (task-actor)
status: completed
type: task
priority: normal
created_at: 2026-09-04T23:29:47Z
updated_at: 2026-09-04T23:43:56Z
---

Implement OpenSpec change task-actor: per-task actor owns TaskDetails, TaskHandle/TaskWriter, registry, state machine wiring, output streaming via watch. Refs #894.

## Summary of Changes

- New `scotty/src/tasks/actor.rs`: per-task actor (mailbox + watch snapshots), `TaskWriter`, `TaskHandle` (Drop fails an unterminated task), `Outcome`/`FailureKind`.
- `TaskManager` is a registry of snapshot receivers; `start_process` reports output and exit code only and returns a JoinHandle; cleanup is TTL on terminal snapshots.
- `Context` owns the `TaskHandle`; `run_sm` drops the machine handle; `TaskCompletionHandler` terminates once (refresh failure -> Failed, missing compose file -> ok); `run_task_and_wait` awaits the subprocess and never changes task state.
- Nested rebuild/purge machines (`nested: true`) have no completion handler.
- Output streaming follows the watch channel and ends after the terminal snapshot.
- scottyctl fails on `state == Failed` regardless of exit code.
- Ported Docker regression test for #894; unit tests for actor, manager, run_sm, completion handler, streaming.
