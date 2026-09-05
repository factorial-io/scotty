## Why

With task ownership moved into the per-task actor (`task-actor`), the `StateMachine` no longer carries any guarantee: its states are never persisted, inspected or resumed, every machine is a straight line with one error edge, and its indirection (state enum + handler struct + `add_handler` per step, `Arc<RwLock<Context>>`, the `nested` flag for create/destroy, an error handler whose own error is swallowed) is now the main source of accidental complexity in the app lifecycle code. An operation should read top to bottom as one async function where `?` is the error edge.

## What Changes

- Replace the seven `*_prepare` state machines (create, destroy, rebuild, purge, run, stop, custom action) with one plain `async fn` each that calls step functions in sequence.
- Replace `run_sm` and `TaskCompletionHandler` with one `run_operation` wrapper that spawns the operation, refreshes app data, terminates the task exactly once and sends the notification.
- Turn each `StateHandler` struct in `docker/state_machine_handlers/` into a step function `async fn step(ctx: &Context, ...) -> anyhow::Result<()>`.
- Nested operations become plain calls (`create` awaits `rebuild_steps`, `destroy` awaits `purge_steps`); the `nested: bool` parameters and `join_outcome` are removed.
- `Context` is passed as `&Context` (it is never mutated); `Arc<RwLock<Context>>` disappears.
- Delete `scotty/src/state_machine.rs`, the seven state enums and the handler structs.
- Log each step in an `info_span!` so the "Running handler for state X" trace is replaced by an equivalent span.

No user-visible behavior changes: task states, status lines, notifications, REST and WebSocket payloads stay the same.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This is a pure refactor; the requirements in `task-lifecycle` (from the `task-actor` change) are preserved, and `.openspec.yaml` sets `skip_specs: true`.

## Impact

- `scotty/src/docker/{create_app,destroy_app,rebuild_app,purge_app,run_app,stop_app,run_app_custom_action}.rs`: rewritten as linear functions.
- `scotty/src/docker/state_machine_handlers/*`: handler structs become step functions (module renamed to `docker/steps/`).
- `scotty/src/docker/helper.rs`: `run_sm` and `join_outcome` replaced by `run_operation`.
- `scotty/src/state_machine.rs`: deleted, including its tests.
- Public API of the `docker` module (`create_app`, `destroy_app`, ... returning `RunningAppContext`) is unchanged, so REST handlers are untouched.
- Tests: unit tests in `helper.rs` and `task_completion_handler.rs` are re-targeted at `run_operation`; the Docker regression test `test_scoped_create_visibility` stays as the end-to-end check.
- kenkeep node `map-state-machine-errors-bubble-to-task-failure` and the `apps/lifecycle` index need an update.
