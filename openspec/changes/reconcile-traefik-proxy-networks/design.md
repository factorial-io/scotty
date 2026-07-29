## Context

See proposal.md — Why for the motivation and the production incident.

Current state:

1. `EnsureAppNetworkHandler` (`scotty/src/docker/state_machine_handlers/network_handler.rs`) runs before `docker compose up`. It creates `<base>--<app>` via `create_network` with labels `scotty.managed=true` and `scotty.app=<name>`, then `connect_network`s the container named by `settings.traefik.container_name` (default `traefik`). Both calls already tolerate 409/403/404, so they are idempotent.
2. `TeardownAppNetworkHandler` reverses this on destroy/purge (`disconnect_network` with `force`, then `remove_network`).
3. `app_proxy_network_name(base, app)` (`scotty/src/docker/loadbalancer/mod.rs`) is the single place that builds `<base>--<app>`.
4. `schedule_app_check` (`scotty/src/docker/setup.rs`) runs once at startup and then on `settings.scheduler.running_app_check`. It calls `find_apps`, stores the result in `app_state.apps`, and broadcasts `AppListUpdated`. `find_apps` returns `AppDataVec` with per-service `ContainerState` (status, domains) and `AppData.settings.public_services`.
5. `traefik.rs` writes `traefik.docker.network=<base>--<app>` on public services and declares the per-app network as `external: true` in the generated override — so the network must pre-exist, and Traefik resolves the backend IP on that network only.

Constraints that shape the design:

- `AppState::docker` is a plain `bollard::Docker` (0.21). No wrapper to extend; calls are made directly, as in the existing handlers.
- Nothing in `AppData` records which Docker networks a container is on, and adding it would touch `ContainerState` and the generated TS bindings.
- Apps created before per-app networks still route over the shared base network (see kenkeep `practice-traefik-network-migration-requires-rebuild`); they must not be "repaired" into a per-app network.
- The app check is a scheduled job whose failure is only logged; it must not become a source of hard failures.

## Goals / Non-Goals

**Goals:**

- Converge Traefik's network membership from observed Docker state on every app check, with zero write calls when already converged.
- Derive the work set from networks that actually exist, not from names computed for every app, so legacy and half-deployed apps are naturally excluded.
- Bound the number of Docker API calls per pass: 1 container inspect + 1 network list + 1 inspect per *candidate orphan* only.
- Keep all decision logic in pure functions that are unit-testable without a Docker daemon.

**Non-Goals:**

- No declarative Traefik network config (the set is dynamic; static declaration in `traefik-compose.yml` does not scale) and no flat shared network (rejected in the issue, see also #848).
- No new configuration keys. Reconciliation is a bug fix, always on for Traefik.
- No migration of legacy shared-network apps (`app:rebuild` remains the migration path).
- No change to `ContainerState`, the REST/WS API, or the generated TS bindings.
- No health endpoint change; drift is surfaced via logs and metrics only.

## Decisions

### Decision 1: Reconcile inside `schedule_app_check`, after the app list is stored

`schedule_app_check` already has the freshly-computed `AppDataVec` and runs at exactly the two moments that matter (startup and every `running_app_check` interval). Reconciliation is added in the `Ok(apps)` arm only, after `set_apps`, and its errors are logged, never propagated:

```rust
Ok(apps) => {
    let _ = app_state.apps.set_apps(&apps).await;
    ...
    if let Err(e) = reconcile_traefik_networks(&app_state, &apps).await {
        tracing::error!("Traefik network reconciliation failed: {:?}", e);
    }
    ...
}
```

Placing it in the `Ok` arm gives the spec's "app discovery failed → no pruning" behavior for free: on a `find_apps` error the pass simply does not run, so a transient discovery failure can never be read as "all apps are gone" and trigger mass pruning.

*Alternatives considered:* a separate `clokwerk` job — rejected, it would need its own `find_apps` call and could observe a stale/partial app list. A Docker event listener on Traefik container start — rejected as the primary mechanism: it needs a long-lived event stream with reconnect logic, and it misses the case where Scotty itself restarts; it is a possible later optimization on top of polling.

### Decision 2: Derive the desired set from existing labelled networks, not from app names

The reconciler builds its work set from Docker, in three reads:

1. `inspect_container(&settings.traefik.container_name, None::<InspectContainerOptions>)` → `NetworkSettings.Networks` keys = `attached: HashSet<String>`.
2. `list_networks` filtered by `label=scotty.managed=true` → candidate per-app networks, each carrying `scotty.app` in its labels.
3. `inspect_network` — only for candidates classified as possible orphans, to read the attached container set.

Each candidate network is then classified against the app list:

- `scotty.app` matches a discovered app **with at least one running container** → **desired**; connect if not in `attached`.
- `scotty.app` matches a discovered app with no running containers → **ignore** (spec: stopped apps are not attached).
- `scotty.app` matches no discovered app → **orphan candidate** → inspect, prune only if no container other than Traefik is attached.

Deriving from existing networks (rather than computing `app_proxy_network_name` for all apps) means: legacy shared-network apps have no labelled per-app network and are skipped; no network is ever created here; and a manually-deleted app directory is detected as an orphan rather than silently re-attached.

Name-based fallback: networks whose name matches `<base>--<app>` for a discovered running app but which lack the labels (created by a version where labelling differed) are also treated as **desired** for the *connect* side. Pruning stays restricted to labelled networks — connecting is cheap and reversible, removing is not. Parsing the app name back out of a network name is not injective when the base contains `--` (documented on `app_proxy_network_name`), so the reverse direction compares against `app_proxy_network_name(base, &app.name)` for each app instead of splitting the network name.

*Alternatives considered:* record each container's networks on `ContainerState` and diff from the app list — rejected, it changes a shared type and the TS bindings for data only this loop needs. Compute `app_proxy_network_name` for every app and blindly `connect` — rejected, it issues a write call per app per pass and would create/attach networks for legacy and stopped apps.

### Decision 3: Guard pruning on "no non-Traefik endpoint attached"

`inspect_network(...).containers` lists attached endpoints. A network is pruned only when that set is empty or contains nothing but the Traefik container. This makes the dangerous half of the reconciler self-limiting: any app whose containers are still up (including one whose directory read or inspection failed in a way that dropped it from `find_apps`) keeps its containers attached and is therefore protected. `disconnect_network(force)` then `remove_network` reuse the exact tolerance pattern already in `TeardownAppNetworkHandler`; a 409 on removal means an endpoint appeared meanwhile, and is logged and skipped.

*Alternatives considered:* prune only networks older than some age, or behind a config flag — rejected as extra surface; the endpoint guard is a stronger and simpler invariant. Never prune at all — rejected, membership growth is in scope per the issue.

### Decision 4: Status tolerance for "running"

"Has at least one running container" uses the existing `ContainerState::is_running()` (`Running | Created | Restarting`) rather than strict `Running` or the app-level `AppStatus`. A `Restarting` or `Created` container is one that Traefik must be able to reach imminently, and an app in `AppStatus::Starting` is exactly the state a partially-recovered host is in. Being generous here only risks an extra harmless attachment; being strict risks leaving an app unroutable.

### Decision 5: Errors are per-app, the pass is not

The reconciler collects per-app failures instead of returning on the first one:

- Traefik container inspect fails (missing container / daemon error) → log `error!` once, count every app with public services as unroutable, return `Ok(())`. The check must still complete.
- A single `connect_network` fails → `error!` with app, network and reason; increment the unroutable counter; continue.
- `list_networks` fails → `error!`, skip the pass.

Only genuinely unexpected conditions return `Err`, which `schedule_app_check` logs.

### Decision 6: Reporting — warn on repair, metrics on both counts

Log lines carry the app name and network so the incident is greppable:

- `warn!` before each repair: "app X was not reachable by Traefik (network N missing from Traefik); reconnecting".
- `info!` on successful repair, on each prune, and on each skipped orphan.
- `error!` when an app with public services stays unroutable.
- A converged pass logs at `debug!` only.

Two metrics are added to `MetricsRecorder` (`recorder_trait.rs`, `otel_recorder.rs`, `noop.rs`) and recorded at the end of every pass, including with value 0, so absence-of-data is distinguishable from zero:

- `scotty_traefik_network_drift_apps` (gauge): apps found drifted in this pass.
- `scotty_traefik_unroutable_apps` (gauge): apps with public services still unroutable after this pass — the alertable signal, since a sustained non-zero value is the outage.

Whether an app "has public services" comes from `app.settings.public_services` being non-empty; apps without settings are reconciled but never counted as unroutable.

### Decision 7: Module placement

New module `scotty/src/docker/loadbalancer/network_reconciler.rs`, registered in `loadbalancer/mod.rs` next to `app_proxy_network_name`, which it shares with the state machine handlers. Public surface is one entry point plus pure helpers:

```rust
pub async fn reconcile_traefik_networks(app_state: &SharedAppState, apps: &AppDataVec) -> anyhow::Result<()>;

// pure, unit-tested without Docker
enum NetworkVerdict { Desired, Ignore, OrphanCandidate }
fn classify(network: &NetworkSummary, apps: &AppDataVec, base: &str) -> NetworkVerdict;
fn is_prunable(inspected_containers: &[String], traefik: &str) -> bool;
```

The load-balancer-type gate (`settings.load_balancer_type != Traefik → return Ok(())`) mirrors `proxy_network_target` in `network_handler.rs`.

## Risks / Trade-offs

- **Mass pruning if `find_apps` returns a short list (I/O error on the apps root, wrong `root_folder` after a config change)** → three independent guards: reconciliation runs only in the `Ok` arm of the app check; only `scotty.managed`-labelled networks are prunable; and a network with any non-Traefik endpoint is never touched, which covers every app whose containers are up. Worst realistic case is removing an empty network belonging to a stopped, undiscovered app — recreated on the next `app:run`.
- **Traefik accumulating network interfaces** → attachments are bounded by "running apps with an existing per-app network", which is the same set the deploy path would attach anyway, and pruning removes the tail. No change to steady-state interface count.
- **Reconciliation racing a deploy or destroy** → all writes are idempotent and version-tolerant (403/404/409 handled), matching the existing handlers; a network that vanishes mid-pass yields a tolerated 404.
- **Per-pass Docker API cost on hosts with many apps** → 2 reads plus 1 read per orphan candidate; the common converged case does zero writes. `running_app_check` already inspects every container of every app, so this is marginal.
- **Traefik discovering a backend before the app's containers join the network** → not a new risk: Traefik re-reads container labels on Docker events, and the `traefik.docker.network` label pins resolution to the per-app network.
- **A legacy unlabelled per-app network is connected but never pruned** → intentional asymmetry; it leaks at most one network per legacy app, cleaned up by destroy/purge.
- **Metrics recorded per pass, not per app** → the gauges say how many apps are affected, not which; the logs carry the names. Adding an app label was rejected to avoid unbounded metric cardinality.

## Migration Plan

- No data migration and no config change. On first start after the upgrade, the startup app check repairs a host that is already broken; orphaned `proxy--*` networks left behind by earlier versions are pruned on the same pass.
- Deployment note for the incident host: the manual `docker network connect proxy--<app> traefik` workaround is no longer needed and is idempotent with the reconciler.
- Rollback: revert the commit. Nothing persists state; membership simply stops being reconciled and reverts to deploy-time-only behavior.

## Open Questions

- Should reconciliation additionally hook Docker events (`container start` for the Traefik container) to cut the repair window from up to one `running_app_check` interval down to seconds? Deferrable: it is an optimization layered on the same reconciler, and it changes neither the specs nor the task breakdown.
