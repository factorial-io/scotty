## Context

Today status flows like this:

1. `inspect_docker_container` (`scotty/src/docker/find_apps.rs`) reads `state.status` into `ContainerState.status` (a `ContainerStatus`). It does **not** read `State.ExitCode`.
2. `get_app_status_from_services` (`scotty-core/src/apps/app_data/status.rs`) folds the per-container statuses into one `AppStatus`:
   ```rust
   match count_running_services {
       0 => AppStatus::Stopped,
       x if x == services.len() => AppStatus::Running,
       _ => AppStatus::Starting,
   }
   ```
   Only `ContainerStatus::Running` is counted, so any non-running container (including a one-shot init container that has `Exited 0`) drops the app to `Starting`.
3. The frontend `app-service-button.svelte` hides the URL with `{#if status !== 'Running'}` where `status` is the **app** status passed by callers in `routes/dashboard/**`, not the service's own status.

Net effect: an app with a normal init container can never reach `Running`, so the running web service's URL stays disabled. The backend already returns per-service status and domains; the UI just isn't using them for gating.

## Goals / Non-Goals

**Goals:**
- A running service shows its clickable URL even when the app as a whole is not `Running`.
- A successfully-completed one-shot/init container (`Exited` with code 0) does not keep the app at `Starting`.
- Distinguish a clean one-shot exit from a crash (non-zero exit / `Dead`).
- Keep the change minimal and localized; no new dependencies.

**Non-Goals:**
- No Kubernetes-style declarative "init container" model or compose-label opt-in (deferred — see Open Questions).
- No health-check (`State.Health`) integration; we continue to use `State.Status` plus the new exit code only.
- No change to load-balancer/Traefik label parsing or how domains are discovered.

## Decisions

### Decision 1: Carry exit code on `ContainerState`

Add `exit_code: Option<i64>` to `ContainerState` (populated from Bollard `state.exit_code`). Bollard exposes `ContainerState.exit_code` as `i64`. Keep it `Option` so existing constructors/tests and any path lacking inspection data stay valid.

Add a derived classification helper rather than a new enum variant, to avoid churn on the `ContainerStatus`↔Bollard `From` mapping and the generated TS bindings:
```rust
impl ContainerState {
    pub fn is_running(&self) -> bool { /* Running | Created | Restarting (unchanged) */ }
    pub fn is_completed(&self) -> bool {
        self.status == ContainerStatus::Exited && self.exit_code == Some(0)
    }
}
```

*Alternative considered:* add a `Completed` variant to `ContainerStatus`. Rejected — `ContainerStatus` mirrors Docker's `ContainerStateStatusEnum` 1:1; injecting a synthetic variant breaks that contract and the round-trip mapping. A derived predicate keeps the raw Docker status intact for display.

### Decision 2: Rework `get_app_status_from_services`

Replace the "all-or-nothing" count with classification-aware aggregation:
```rust
// strict `Running` (not is_running(), which also counts Created/Restarting) to
// preserve the original "Created => Starting" semantics — this is a fix, not a
// behavioral expansion.
let running   = count_state(services, ContainerStatus::Running);
let completed = services.iter().filter(|s| s.is_completed()).count();
match (running, completed) {
    (0, 0) => AppStatus::Stopped,
    (r, c) if r > 0 && r + c == services.len() => AppStatus::Running,
    (0, _) => AppStatus::Stopped,            // only completed one-shots, nothing live
    _ => AppStatus::Starting,
}
```
Rationale: an app is `Running` once everything that should be live is live and the rest finished cleanly. A failed container leaves the app in `Starting` (it is neither running nor completed), which is the honest signal — but per Decision 3 the healthy services still surface their URLs.

*Alternative considered:* mark app `Running` if **any** container runs. Rejected — too lax; a half-started app would look healthy. We require all containers to be running-or-completed.

### Decision 3: Gate URL visibility on per-service status (frontend)

`app-service-button.svelte` already receives the `service` object (which has `status`). Change the guard from the passed-in app `status` to `service.status`:
```svelte
{#if service.status !== 'Running'}
  <button class="btn btn-xs" disabled>{service.service}</button>
{:else}
  ... clickable domain links ...
{/if}
```
Drop the `status` prop (or keep it optional for backward compat) and update the three callers in `routes/dashboard/+page.svelte`, `routes/dashboard/[slug]/+page.svelte`, and `routes/dashboard/[slug]/[service]/+page.svelte`. Add `status` to the `AppService` interface in `frontend/src/types.ts` if not already present (it is present).

### Decision 4: Keep exit code out of the URL gate

URL visibility keys off `service.status === 'Running'` only. A completed init container has no web URL anyway (no domains, or it's exited), so it naturally renders as disabled — no special-casing needed in the button.

## Risks / Trade-offs

- **[A long-running service legitimately exits 0 and we now call the app `Running`]** → Only when at least one sibling is still `Running`; an app where everything has exited becomes `Stopped`. Acceptable: a single Exited-0 service among running ones is, by definition, the one-shot case we want to tolerate.
- **[Crash loops]** → A container crash-looping shows `Restarting`, which `is_running()` already treats as running (unchanged behavior); a hard `Exited` non-zero correctly leaves the app at `Starting` and that service's URL disabled.
- **[Generated TS bindings drift]** → Adding `exit_code` to `ContainerState` requires regenerating the ts-rs bindings and updating the hand-maintained `frontend/src/types.ts`. Covered as an explicit task.
- **[Existing tests asserting `Starting` for init-container apps]** → Update those assertions; the new behavior is the intended outcome.

## Migration Plan

- Pure behavioral change, no data migration. Status is recomputed live on each inspection.
- Rollback: revert the commit; no persisted state depends on the new field.

## Open Questions

- Should scotty later support an explicit one-shot marker (compose label / `restart: "no"`) to override the exit-code heuristic for services that legitimately exit non-zero? Deferred per the chosen exit-code-0 approach; can be a follow-up bean.
