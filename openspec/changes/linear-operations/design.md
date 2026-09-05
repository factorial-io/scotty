## Context

See proposal.md for motivation. After `task-actor`, the state machine `Context` owns the `TaskHandle`; `run_sm` spawns the machine and drops its `JoinHandle`, and `TaskCompletionHandler` is the only place that terminates the task. Seven operations are built by `*_prepare` functions that register `StateHandler` structs under state enum variants. Create and destroy nest rebuild and purge via a `nested: bool` flag that suppresses the inner machine's completion handler and error state. Handlers receive `Arc<RwLock<Context>>` but never write to it.

Constraints: the public functions `create_app`, `destroy_app`, `rebuild_app`, `purge_app`, `run_app`, `stop_app`, `force_stop_app`, `run_app_custom_action` keep their signatures (they return `RunningAppContext` immediately); status lines, notifications and task states must stay byte-identical so the frontend, scottyctl and the Docker regression test are unaffected.

## Goals / Non-Goals

**Goals:**
- Every operation is one `async fn` readable top to bottom; `?` is the only error edge.
- Exactly one place terminates a task and sends the notification.
- Nesting is a function call.
- Delete `state_machine.rs` and the handler indirection without changing observable behavior.

**Non-Goals:**
- Changing which steps an operation runs, their order, timeouts or messages.
- Making steps resumable, persistent or observable as states.
- Touching the actor, `TaskManager` or the read path.

## Decisions

### D1: `async fn` per operation, step functions instead of handler structs

Each `*_prepare` becomes `async fn <op>_steps(ctx: &Context, ...) -> anyhow::Result<()>`; each handler struct becomes `pub async fn <step>(ctx: &Context, <former fields>) -> anyhow::Result<()>` in `docker/steps/`. The `next_state` fields disappear because sequencing is the function body.

Alternatives: keep `StateMachine` with a builder to reduce ceremony (still spreads control flow over a map); a generic `Vec<Box<dyn Step>>` pipeline (loses per-step argument types and gains nothing over sequential `await`s). Rust's `async fn` already compiles to a state machine, so a hand-written one on top only duplicates it.

### D2: One `run_operation` wrapper replaces `run_sm` and `TaskCompletionHandler`

```rust
pub async fn run_operation<F>(
    app_state: SharedAppState,
    app: &AppData,
    notification: Option<Message>,
    op: impl FnOnce(Arc<Context>) -> F + Send + 'static,
) -> anyhow::Result<RunningAppContext>
where F: Future<Output = anyhow::Result<()>> + Send
```

It creates the `Context` (which creates the task), writes the "Starting app" status line, returns the `RunningAppContext`, and spawns:

1. `let result = op(ctx.clone()).await;`
2. `let refresh = refresh_app_data(&ctx).await;` (skipped when the compose file no longer exists, as today)
3. `ctx.task.terminate(outcome(result, refresh))` where a refresh error wins over success (`FailureKind::RefreshFailed`), a step error is `StepFailed`, otherwise `Finished`.
4. Send `notification` only on `Finished`.

The `TaskHandle::Drop` fallback still covers a panic inside `op`. The `Context` is shared as `Arc<Context>` (no `RwLock`), because nothing mutates it.

Alternative: keep `TaskCompletionHandler` as a function called at the end of each operation. Rejected: it would have to be called in every error path of every operation, which is exactly the class of bug `task-actor` closed.

### D3: Nested operations are plain calls

`create_steps` calls `rebuild_steps(ctx, recreate_lb = false).await?`; `destroy_steps` calls `purge_steps(ctx, PurgeAppMethod::Down).await?`. Because `*_steps` never terminate the task or notify (only `run_operation` does), the `nested` flag and `join_outcome` are unnecessary. `rebuild_app` is then `run_operation(app_state, app, Some(AppRebuilt), |ctx| rebuild_steps(ctx, true))`.

### D4: Step failures name the step

`run_operation` wraps each operation's error with the failing step via `anyhow::Context` in the step functions (`.context("docker compose up")`), and the terminal status line becomes `Operation failed for app '<name>': <error chain>`. Today the line carries no cause because `StateMachine::run` swallows the error before `TaskCompletionHandler` runs. This is the one deliberate message change; scottyctl and the frontend do not parse it, and the Docker test only asserts the success line.

### D5: Tracing per step

`StateMachine::run` logged every transition. Each step function is annotated with `#[instrument(skip(ctx), fields(app = %ctx.app_data.name))]`, which yields the same information as spans, plus timing.

### D6: Module layout

`docker/state_machine_handlers/` is renamed to `docker/steps/`; `context.rs` moves with it. `run_task_and_wait` stays as the shared subprocess step. `helper.rs` keeps `wait_for_containers_ready` and gains `run_operation`.

## Risks / Trade-offs

- [A step is accidentally left out or reordered during the port] → Port one operation per commit; for each, diff the ordered list of status lines from the old machine (captured by the Docker test for create/destroy and by `docker compose` unit tests for the others) against the new function.
- [Behavior difference in the failure status line (D4)] → Intentional; documented here and in the kenkeep node. Consumers only branch on `state`.
- [`run_operation` closure type gymnastics obscure the code] → Keep the signature simple (`FnOnce(Arc<Context>) -> impl Future`), one call site per operation; if the bound becomes noisy, box the future.
- [Loss of the generic `StateMachine` for future non-linear flows] → None exist today; if one appears, an `async fn` with a `loop`/`match` still expresses it.

## Migration Plan

Single PR after `task-actor` lands, one commit per operation plus a final deletion commit, so a regression can be bisected to one operation. No deployment or data migration; rollback is reverting the PR.
