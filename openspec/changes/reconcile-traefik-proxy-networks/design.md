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
- Nothing in `AppData` records which Docker networks a container is on. `AppData` is rebuilt from scratch by `find_apps` on every check, so any connectivity field on it has to be written during the check rather than persisted.
- `AppData` is the scottyctl wire type as well as the frontend's; per kenkeep `practice-wire-format-changes-must-be-backward-compatible-in-both-directions`, a new field must be optional in both directions (preflight only gates major.minor).
- `info_app_handler` and `apps/list` serve from the `app_state.apps` cache, so annotating the cached `AppData` is enough to expose connectivity through the API. State-machine paths (`run_app`, `rebuild_app`, …) return freshly-inspected `AppData` that has not been through a reconciliation pass.
- Apps created before per-app networks still route over the shared base network (see kenkeep `practice-traefik-network-migration-requires-rebuild`); they must not be "repaired" into a per-app network.
- The app check is a scheduled job whose failure is only logged; it must not become a source of hard failures.
- The frontend is tightly coupled to the backend and needs no compatibility shim (kenkeep `practice-frontend-backend-tight-coupling`), but `types.ts` is hand-maintained, not generated from `AppData`.

## Goals / Non-Goals

**Goals:**

- Converge Traefik's network membership from observed Docker state on every app check, with zero write calls when already converged.
- Repair a recreated Traefik within seconds, not within one check interval, without making the event stream load-bearing.
- Derive the work set from networks that actually exist, not from names computed for every app, so legacy and half-deployed apps are naturally excluded.
- Bound the number of Docker API calls per pass: 1 container inspect + 1 network list + 1 inspect per *candidate orphan* only.
- Keep all decision logic in pure functions that are unit-testable without a Docker daemon.
- Make the state the reconciler observed visible in the API and UI, rather than only in logs.

**Non-Goals:**

- No declarative Traefik network config (the set is dynamic; static declaration in `traefik-compose.yml` does not scale) and no flat shared network (rejected in the issue, see also #848).
- No configuration for reconciliation itself. Only event watching is configurable; periodic reconciliation is a bug fix and always on for Traefik.
- No migration of legacy shared-network apps (`app:rebuild` remains the migration path).
- No change to `ContainerState` or the ts-rs generated bindings; connectivity is app-level, not per-service.
- No connectivity column or badge on the dashboard app list — decided, not deferred; the detail-page indicator plus the metric is the whole UI surface. The list response carries the field, so adding one later is presentation-only.
- No active HTTP probing of app URLs — the indicator reflects Docker network membership only.
- No health endpoint change; the machine-readable signals are the metrics and the API field.

## Decisions

### Decision 1: Reconcile inside `schedule_app_check`, before the app list is published

`schedule_app_check` already has the freshly-computed `AppDataVec` and runs at exactly the two moments that matter (startup and every `running_app_check` interval). Reconciliation is added in the `Ok(apps)` arm only, *before* `set_apps`, so the same pass that repairs membership also annotates each `AppData` with the connectivity it observed and the cache is published once, already annotated:

```rust
Ok(mut apps) => {
    // annotates apps[i].load_balancer_connectivity as a side effect
    if let Err(e) = reconcile_traefik_networks(&app_state, &mut apps).await {
        tracing::error!("Traefik network reconciliation failed: {:?}", e);
    }
    let _ = app_state.apps.set_apps(&apps).await;
    ...
}
```

Being inside the `Ok` arm is what gives the spec's "app discovery failed → no pruning" behavior for free: on a `find_apps` error the pass does not run at all, so a transient discovery failure can never be read as "all apps are gone" and trigger mass pruning.

*Alternatives considered:* a separate `clokwerk` job — rejected, it would need its own `find_apps` call and could observe a stale/partial app list. Reconciling after `set_apps` — rejected, it would publish the app list once without connectivity and again with it, so the UI would flicker through `Unknown` on every check.

### Decision 2: Docker events as an accelerator on top of polling, never as the mechanism

The periodic pass stays authoritative; the event watcher only makes it fire sooner. A dedicated task spawned from `setup_docker_integration` subscribes to `docker.events(...)` filtered server-side to `type=container`, `event=start`, `container=<traefik.container_name>`, and on each matching event triggers a reconciliation pass. It runs only when the load balancer is Traefik *and* `traefik.watch_docker_events` is true.

Structure, mirroring the scheduler task's `stop_flag` loop:

```rust
while !stop_flag.is_stopped() {
    match docker.events(Some(opts.clone())) { /* stream */ }
    // on stream end/error: log, backoff, resubscribe
    // on resubscribe: reconcile once, since a start may have been missed while disconnected
}
```

Three properties matter:

- **Reconcile on (re)subscribe**, not only on events. A container start that happens while the stream is down would otherwise be missed until the next scheduled check.
- **Exponential backoff with a cap** (e.g. 1s → 30s) on stream failure, so a daemon restart does not spin.
- **Coalescing, not per-event work.** Recreating a container emits `die` then `start`, and compose can emit bursts. A short debounce (~2s) plus the single-flight lock from Decision 7 collapses a burst into one pass.

The event-driven pass has no fresh `find_apps` result, so it works from the `app_state.apps` cache (at most one check interval stale — good enough, since a network cannot exist for an app that was never deployed), writes annotations back with `SharedAppList::update_app`, and broadcasts `AppListUpdated` so open UIs refresh. It never prunes on a cache-only pass: pruning decisions require the authoritative on-disk app list, and the cache could omit an app added since the last check. Pruning therefore stays exclusive to the periodic pass.

*Alternatives considered:* events as the only trigger — rejected, it misses Scotty's own restart and any event lost while disconnected. Polling `inspect_container` on Traefik every few seconds — rejected, same latency benefit at a much worse cost. Making the setting an interval rather than a boolean — rejected as unnecessary surface; `running_app_check` already governs the polling cadence.

### Decision 3: `traefik.watch_docker_events` — one boolean, default true

Added to `TraefikSettings` (`scotty-core/src/settings/loadbalancer.rs`) next to `container_name`, following the same pattern:

```rust
pub fn default_traefik_watch_docker_events() -> bool { true }

#[serde(default = "default_traefik_watch_docker_events")]
pub watch_docker_events: bool,
```

with the same value in the `Default` impl, a documented entry in `config/default.yaml` and `config/default.yaml.example`, and the usual override path `SCOTTY__TRAEFIK__WATCH_DOCKER_EVENTS=false`. Default true because the failure it prevents is a silent multi-hour outage; the escape hatch exists for hosts where a long-lived events subscription is undesirable (restricted socket proxy, event-stream noise). Disabling it only widens the repair window to one `running_app_check` interval — it never disables repair.

### Decision 4: Derive the desired set from existing labelled networks, not from app names

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

### Decision 5: Guard pruning on "no non-Traefik endpoint attached"

`inspect_network(...).containers` lists attached endpoints. A network is pruned only when that set is empty or contains nothing but the Traefik container. This makes the dangerous half of the reconciler self-limiting: any app whose containers are still up (including one whose directory read or inspection failed in a way that dropped it from `find_apps`) keeps its containers attached and is therefore protected. `disconnect_network(force)` then `remove_network` reuse the exact tolerance pattern already in `TeardownAppNetworkHandler`; a 409 on removal means an endpoint appeared meanwhile, and is logged and skipped.

*Alternatives considered:* prune only networks older than some age, or behind a config flag — rejected as extra surface; the endpoint guard is a stronger and simpler invariant. Never prune at all — rejected, membership growth is in scope per the issue.

### Decision 6: Status tolerance for "running"

"Has at least one running container" uses the existing `ContainerState::is_running()` (`Running | Created | Restarting`) rather than strict `Running` or the app-level `AppStatus`. A `Restarting` or `Created` container is one that Traefik must be able to reach imminently, and an app in `AppStatus::Starting` is exactly the state a partially-recovered host is in. Being generous here only risks an extra harmless attachment; being strict risks leaving an app unroutable.

### Decision 7: Single-flight — passes never overlap

Two triggers now exist (scheduler and events), so passes can collide. The reconciler holds a `tokio::sync::Mutex<()>` in `AppState` (or a module-level `OnceLock`); an event-driven pass uses `try_lock` and **skips** if a pass is already running, because the in-flight pass will observe the same Docker state anyway. The periodic pass uses `lock().await` so a scheduled pass is never dropped. This also keeps annotation writes to the cached app list serialized.

*Alternatives considered:* an mpsc channel with a single consumer task as the only entry point — cleaner in principle, but it decouples the periodic pass from `schedule_app_check`, which is exactly where the fresh `AppDataVec` lives; the lock keeps the data flow direct.

### Decision 8: Errors are per-app, the pass is not

The reconciler collects per-app failures instead of returning on the first one:

- Traefik container inspect fails (missing container / daemon error) → log `error!` once, count every app with public services as unroutable, return `Ok(())`. The check must still complete.
- A single `connect_network` fails → `error!` with app, network and reason; increment the unroutable counter; continue.
- `list_networks` fails → `error!`, skip the pass.

Only genuinely unexpected conditions return `Err`, which `schedule_app_check` logs.

### Decision 9: Reporting — warn on repair, metrics on both counts

Log lines carry the app name and network so the incident is greppable:

- `warn!` before each repair: "app X was not reachable by Traefik (network N missing from Traefik); reconnecting".
- `info!` on successful repair, on each prune, and on each skipped orphan.
- `error!` when an app with public services stays unroutable.
- A converged pass logs at `debug!` only.

Two metrics are added to `MetricsRecorder` (`recorder_trait.rs`, `otel_recorder.rs`, `noop.rs`) and recorded at the end of every pass, including with value 0, so absence-of-data is distinguishable from zero:

- `scotty_traefik_network_drift_apps` (gauge): apps found drifted in this pass.
- `scotty_traefik_unroutable_apps` (gauge): apps with public services still unroutable after this pass — the alertable signal, since a sustained non-zero value is the outage.

Whether an app "has public services" comes from `app.settings.public_services` being non-empty; apps without settings are reconciled but never counted as unroutable.

### Decision 10: Connectivity is an app-level enum on `AppData`, written by the reconciler

`AppData` gains one field, and a new enum lives next to it in `scotty-core/src/apps/app_data/`:

```rust
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub enum LoadBalancerConnectivity {
    #[default]
    Unknown,             // no reconciliation pass has observed this app yet
    NotApplicable,       // no public services, non-Traefik LB, or legacy shared network
    Connected,
    Disconnected,        // network exists, app running, Traefik not attached
    LoadBalancerUnavailable, // Traefik container missing or not inspectable
}

// on AppData
#[serde(default)]
pub load_balancer_connectivity: LoadBalancerConnectivity,
```

Four things follow from the constraints:

- **An enum, not a `bool` or `Option<bool>`.** "Not applicable", "unknown", and "the load balancer itself is gone" are three different operator actions; collapsing them into `false` would light up the indicator for every app without public services.
- **`Unknown` is the `Default`**, so `AppData::default()`, `AppData::new()`, and every state-machine path that returns freshly-inspected data are correct without touching them — they report "not yet determined", which is honest. Only the reconciler writes a definite value.
- **`#[serde(default)]`** satisfies the both-directions wire rule: a newer scottyctl reading an older server's payload gets `Unknown` instead of a parse error, and an older scottyctl ignores the unknown field.
- **App-level, not per-service.** Network membership is per app; putting it on `ContainerState` would imply per-service granularity that does not exist and would churn the ts-rs bindings.

`NotApplicable` is assigned when the load balancer is not Traefik, when `settings.public_services` is empty, or when the app has no per-app proxy network (the legacy shared-network case) — the same predicate that already excludes those apps from the unroutable metric, so the field and the metric cannot disagree.

*Alternatives considered:* a separate endpoint (`GET /apps/{app}/connectivity`) — rejected, it needs a second round-trip for something already computed in the check that fills the app cache. Deriving connectivity in the handler on demand by inspecting Docker per request — rejected, it puts Docker latency on a read path that is currently a cache lookup.

### Decision 11: Frontend indicator is a pill next to the app status, silent when not applicable

New `frontend/src/components/app-connectivity-pill.svelte`, built on the existing `pill.svelte` the way `app-status-pill.svelte` is, rendered in the app detail page's `PageHeader` `meta` slot next to `<AppStatusPill>`:

| state | rendering |
|---|---|
| `Connected` | green pill, "Routable" |
| `Disconnected` | red pill, "Not routable", `title` explaining the load balancer is not attached to the app's network |
| `LoadBalancerUnavailable` | amber pill, "LB unavailable" |
| `NotApplicable`, `Unknown` | renders nothing |

Rendering nothing for `NotApplicable`/`Unknown` is what keeps the indicator meaningful: an always-present neutral pill trains people to ignore it, and the point of this change is that the broken state is *noticeable*. The detail page already re-derives `data` from the `apps` store subscription, and the reconciler broadcasts `AppListUpdated` after annotating, so the indicator follows live updates with no extra wiring. `frontend/src/types.ts` gains `load_balancer_connectivity: string` on `App` (hand-maintained, not generated).

`scottyctl`'s `format_app_info` gains a "Load balancer" row rendering the same states, so the CLI's app detail view does not contradict the UI's. The `app:list` table is left alone — adding a fifth column for a rarely-set state is not worth the width.

### Decision 12: Module placement

New module `scotty/src/docker/loadbalancer/network_reconciler.rs`, registered in `loadbalancer/mod.rs` next to `app_proxy_network_name`, which it shares with the state machine handlers. Public surface is two entry points plus pure helpers:

```rust
/// Full pass: repairs membership, prunes orphans, annotates `apps`.
pub async fn reconcile_traefik_networks(
    app_state: &SharedAppState,
    apps: &mut AppDataVec,
) -> anyhow::Result<()>;

/// Event-driven pass: works from the app cache, annotates via `update_app`,
/// broadcasts `AppListUpdated`, never prunes.
pub async fn reconcile_from_cache(app_state: &SharedAppState) -> anyhow::Result<()>;

/// Long-lived event watcher task; returns when the stop flag is set.
pub async fn watch_traefik_events(app_state: SharedAppState);

// pure, unit-tested without Docker
enum NetworkVerdict { Desired, Ignore, OrphanCandidate }
fn classify(network: &NetworkSummary, apps: &AppDataVec, base: &str) -> NetworkVerdict;
fn is_prunable(inspected_containers: &[String], traefik: &str) -> bool;
fn connectivity_for(app: &AppData, network: Option<&str>, attached: &HashSet<String>, lb_up: bool)
    -> LoadBalancerConnectivity;
```

Both passes share one internal implementation parameterized by "may prune" and "app source", so the connect and annotate logic cannot diverge between triggers. The load-balancer-type gate (`settings.load_balancer_type != Traefik → return Ok(())`) mirrors `proxy_network_target` in `network_handler.rs`.

## Risks / Trade-offs

- **Mass pruning if `find_apps` returns a short list (I/O error on the apps root, wrong `root_folder` after a config change)** → three independent guards: reconciliation runs only in the `Ok` arm of the app check; only `scotty.managed`-labelled networks are prunable; and a network with any non-Traefik endpoint is never touched, which covers every app whose containers are up. Worst realistic case is removing an empty network belonging to a stopped, undiscovered app — recreated on the next `app:run`.
- **Traefik accumulating network interfaces** → attachments are bounded by "running apps with an existing per-app network", which is the same set the deploy path would attach anyway, and pruning removes the tail. No change to steady-state interface count.
- **Reconciliation racing a deploy or destroy** → all writes are idempotent and version-tolerant (403/404/409 handled), matching the existing handlers; a network that vanishes mid-pass yields a tolerated 404.
- **Per-pass Docker API cost on hosts with many apps** → 2 reads plus 1 read per orphan candidate; the common converged case does zero writes. `running_app_check` already inspects every container of every app, so this is marginal.
- **Traefik discovering a backend before the app's containers join the network** → not a new risk: Traefik re-reads container labels on Docker events, and the `traefik.docker.network` label pins resolution to the per-app network.
- **A legacy unlabelled per-app network is connected but never pruned** → intentional asymmetry; it leaks at most one network per legacy app, cleaned up by destroy/purge.
- **Metrics recorded per pass, not per app** → the gauges say how many apps are affected, not which; the logs carry the names and the API field carries the per-app state. Adding an app label was rejected to avoid unbounded metric cardinality.
- **A long-lived Docker events subscription is a new failure surface** (socket proxies that drop idle streams, daemon restarts, event floods on busy hosts) → the watcher is strictly an accelerator: its task is isolated, its failures are logged and retried with capped backoff, and every repair it performs would happen anyway on the next scheduled pass. `traefik.watch_docker_events=false` disables it outright.
- **Event-driven passes work from a cache that can be up to one interval stale** → they only ever *connect*, never prune, so a stale cache can at worst delay a repair to the next scheduled pass; it cannot cause a wrong removal.
- **Connectivity shows `Unknown` right after a deploy** until the next pass, so the UI briefly shows no indicator for a freshly-created app → acceptable and deliberate: the deploy path itself attaches Traefik, so the honest state is "not yet observed", and `running_app_check` closes the gap. Showing `Connected` optimistically would reintroduce the class of bug this change fixes.
- **The event pass writes the app cache, and the cache has writers that know nothing about the single-flight lock** — every state-machine transition, notification change and custom action calls `update_app` directly. Since the event pass reads the cache, does Docker I/O, and writes afterwards, storing whole `AppData` values back would revert anything those writers changed in that window (caught in review; the lock only serializes the two reconciler entry points against each other). → The event pass patches a single field per app via `SharedAppList::set_load_balancer_connectivity`, which read-modify-writes under one write lock and touches nothing else, and only for apps whose value actually changed. Regression test: `set_load_balancer_connectivity_does_not_revert_concurrent_changes`.
- **The periodic pass still publishes wholesale** via `find_apps` → `set_apps`, which has always clobbered concurrent `update_app` writes; reconciliation adds two Docker reads to a window already dominated by `docker compose ps` plus a per-container inspect. Pre-existing and out of scope here, but worth knowing it is the same shape of race.
- **`AppData` grows a field used by three consumers** (frontend, scottyctl, OpenAPI) → additive and `serde(default)`, so both wire directions stay compatible; the frontend ships in lockstep and needs only the `types.ts` addition.

## Migration Plan

- No data migration. One new config key with a safe default (`traefik.watch_docker_events: true`); existing config files keep working untouched because of the serde default.
- On first start after the upgrade, the startup app check repairs a host that is already broken; orphaned `proxy--*` networks left behind by earlier versions are pruned on the same pass. Thereafter a Traefik recreate is repaired within seconds via the event watcher.
- Deployment note for the incident host: the manual `docker network connect proxy--<app> traefik` workaround is no longer needed and is idempotent with the reconciler.
- Rollback: revert the commit. Nothing persists state; membership stops being reconciled and reverts to deploy-time-only behavior, and the `AppData` field disappears (older clients already tolerate its absence).
