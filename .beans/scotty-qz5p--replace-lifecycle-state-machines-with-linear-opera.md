---
# scotty-qz5p
title: Replace lifecycle state machines with linear operations (linear-operations)
status: completed
type: task
priority: normal
created_at: 2026-09-05T08:14:43Z
updated_at: 2026-09-05T08:24:13Z
---

Implement OpenSpec change linear-operations: run_operation wrapper, step functions, one async fn per operation, delete state_machine.rs. Follows task-actor (#898).

## Summary of Changes

- `helper::run_operation` replaces `run_sm` + `TaskCompletionHandler`: creates the Context, spawns the steps, refreshes app data, terminates the task once (refresh error > step error > finished), notifies on success. `join_outcome` and `nested` removed.
- `docker/state_machine_handlers/` became `docker/steps/` with plain `async fn` steps taking `&Context` (no RwLock): files, compose, network, load_balancer, post_actions, wait_for_containers.
- Each of the seven operations is a `<op>_steps` fn plus the public entry; create calls `rebuild_steps`, destroy calls `purge_steps`.
- Deleted `state_machine.rs`, seven state enums, all handler structs, `TaskHandle::is_terminated`.
- Failure status line now carries the error cause.
- Tests: run_operation (success, step error, panic, refresh failure); Docker regression test passes.
