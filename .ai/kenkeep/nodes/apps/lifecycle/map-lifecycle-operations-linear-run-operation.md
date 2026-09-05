---
type: map
title: Lifecycle operations are linear async fns; run_operation owns the task
description: >-
  Each app operation (create, destroy, rebuild, purge, run, stop, custom action)
  is one async fn sequencing step functions in docker/steps/; helper::run_operation
  owns the per-task actor handle, so errors and panics always end the task Failed
  exactly once, and nesting is a plain function call.
tags:
  - lifecycle
  - tasks
  - docker
kk_schema_version: 3
kk_id: map-lifecycle-operations-linear-run-operation
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---

Each lifecycle operation lives in `scotty/src/docker/<op>_app.rs` as a pair: `<op>_steps(ctx: &Context, ...) -> anyhow::Result<()>` sequences step functions from `scotty/src/docker/steps/` with `?`, and the public `<op>_app(...)` validates, then hands the steps to `helper::run_operation`. There is no state machine: Rust's `async fn` is the state machine, and `?` is the only error edge.

`run_operation` creates the `Context` (which creates the task actor, see `scotty/src/tasks/actor.rs`), writes the "Starting app" status line, returns the `RunningAppContext` immediately, and spawns the steps. When they return it refreshes app data (skipped when the compose file is gone, e.g. after destroy), terminates the task exactly once via the context's `TaskHandle`, and sends the notification only on success. The outcome precedence is: refresh error (`FailureKind::RefreshFailed`) > step error (`StepFailed`, status line carries the error chain) > `Finished`. A panic in a step drops the context and `TaskHandle::Drop` fails the task with "aborted unexpectedly".

Nesting is a call: `create_steps` awaits `rebuild_steps(ctx, false)`, `destroy_steps` awaits `purge_steps(ctx, Down)`. Step functions never terminate the task or notify, so there is no "nested" mode.

Adding a step: write `pub async fn step(ctx: &Context, ...) -> anyhow::Result<()>` in `docker/steps/`, annotate with `#[instrument(skip_all, fields(app = %ctx.app_data.name))]`, write task output through `ctx.task.writer()`, and call it from the operation's `*_steps`. Never call `ctx.task.terminate` from a step.
