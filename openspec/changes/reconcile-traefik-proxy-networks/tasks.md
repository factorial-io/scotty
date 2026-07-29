## 1. Types and configuration

- [x] 1.1 Add `LoadBalancerConnectivity` (`Unknown` as `Default`, `NotApplicable`, `Connected`, `Disconnected`, `LoadBalancerUnavailable`) in `scotty-core/src/apps/app_data/`, deriving `Serialize`/`Deserialize`/`Clone`/`PartialEq`/`ToSchema`, and export it from `app_data/mod.rs`
- [x] 1.2 Add `#[serde(default)] pub load_balancer_connectivity: LoadBalancerConnectivity` to `AppData`, updating `Default` and `AppData::new` so every existing construction site keeps compiling and reports `Unknown`
- [x] 1.3 Add `watch_docker_events: bool` to `TraefikSettings` with `default_traefik_watch_docker_events() -> bool { true }`, wire it into the `Default` impl and `TraefikSettings::new`, and check for callers of `new` that need updating
- [x] 1.4 Document the key in `config/default.yaml` and `config/default.yaml.example` (with the `SCOTTY__TRAEFIK__WATCH_DOCKER_EVENTS` override noted), and confirm existing config files without the key still load

## 2. Reconciler scaffolding

- [x] 2.1 Add `scotty/src/docker/loadbalancer/network_reconciler.rs`, register it in `loadbalancer/mod.rs`, and define the shared internal pass parameterized by "may prune" and app source, returning `Ok(())` immediately when `settings.load_balancer_type != LoadBalancerType::Traefik`
- [x] 2.2 Move/share the "is Traefik + resolve container name + base network" lookup so the reconciler and `state_machine_handlers/network_handler.rs::proxy_network_target` cannot drift apart
- [x] 2.3 Add the single-flight guard (module-level `LazyLock<tokio::sync::Mutex<()>>` rather than an `AppState` field, avoiding churn in two constructors): periodic passes `lock().await`, event-driven passes `try_lock` and skip
- [x] 2.4 Confirm the bollard 0.21 signatures for `list_networks`, `inspect_network`, and `events` (filters) against the crate. Resolved: list networks *unfiltered* and read `scotty.managed`/`scotty.app` per network, since the classifier also needs unlabelled per-app networks (3.3) and "does this app's network exist" — one call serves all three, and the label still gates pruning

## 3. Observation

- [x] 3.1 Read Traefik's current membership: `inspect_container(container_name, None::<InspectContainerOptions>)` → `NetworkSettings.Networks` keys as a `HashSet<String>`; on failure log `error!` once, mark apps with public services `LoadBalancerUnavailable`, count them unroutable, and return `Ok(())`
- [x] 3.2 List candidate networks via `list_networks`, capturing each network's name, `scotty.app` label and managed flag; on failure log `error!` and skip the pass
- [x] 3.3 Add the name-based fallback set: for each discovered app, `app_proxy_network_name(base, &app.name)` matched against existing network names, so unlabelled per-app networks are still connect-eligible (never prune-eligible)
- [x] 3.4 Gate the membership read on liveness: return the attachment set only when `State.Running == Some(true)`, and treat a non-running container exactly like an absent one (the unavailable path from 3.1). Docker retains `NetworkSettings.Networks` on a stopped container, so without this an exited Traefik reads as attached and every running app is reported `Connected` — see design Decision 13. One guard in `traefik_networks`, so all three triggers inherit it

## 4. Classification (pure, unit-testable)

- [x] 4.1 Implement `classify(...) -> NetworkVerdict` with `Desired` (app discovered and has a container where `ContainerState::is_running()`), `Ignore` (app discovered, nothing running), `OrphanCandidate` (no matching discovered app)
- [x] 4.2 Implement `is_prunable(inspected_containers, traefik_name) -> bool`: true only when no attached endpoint other than the Traefik container remains
- [x] 4.3 Implement the "has public services" predicate from `app.settings.public_services` (apps without settings are reconciled but never counted unroutable)
- [x] 4.4 Implement `connectivity_for(...) -> LoadBalancerConnectivity`, using the same public-services predicate so the field and the unroutable metric can never disagree
- [x] 4.5 Unit-test 4.1–4.4 with hand-built `AppDataVec`/network fixtures: running app connected, running app disconnected, stopped app, unknown app, legacy app with no per-app network, app with no public services, unlabelled per-app network, base network itself, network with a foreign container attached, Traefik container missing
- [x] 4.6 Add the stopped-load-balancer case to the 4.5 fixtures: a running app with public services whose per-app network *is* in the attachment set, but with the load balancer not running, reports `LoadBalancerUnavailable` and never `Connected`. This is the regression that a green "Routable" pill in front of a dead Traefik would otherwise re-introduce

## 5. Convergence actions

- [x] 5.1 Connect: for each `Desired` network absent from Traefik's membership, `warn!` naming app and network, `connect_network`, `info!` on success, tolerating 403/409 (already connected) and 404 (network/container vanished mid-pass) exactly as `EnsureAppNetworkHandler` does
- [x] 5.2 Prune (periodic pass only): for each `OrphanCandidate`, `inspect_network`, and when `is_prunable`, `disconnect_network` with `force` then `remove_network`, tolerating 403/404/409; `info!` each prune and each skipped orphan with the reason
- [x] 5.3 Annotate every app in the pass with its `LoadBalancerConnectivity`, computed after the connect attempts so a failed repair reads `Disconnected`, not `Connected`
- [x] 5.4 Ensure no single-app failure aborts the pass: accumulate failures, keep iterating, and never create a network in this code path

## 6. Docker event watcher

- [x] 6.1 Implement `watch_traefik_events(app_state)`: subscribe to `docker.events` filtered to `type=container`, `event=start`, `container=<traefik.container_name>`, looping until `stop_flag.is_stopped()`
- [x] 6.2 Reconcile once on every successful (re)subscribe, so a container start missed while the stream was down is still repaired
- [x] 6.3 Add capped exponential backoff (1s → 30s) on stream end/error with a single `warn!` per reconnect cycle, so a daemon restart cannot spin
- [x] 6.4 Debounce bursts (~2s coalescing window) so a `die`+`start` recreate triggers one pass, not several
- [x] 6.5 Implement `reconcile_from_cache`: work from `app_state.apps.get_apps()`, connect only (never prune), patch connectivity back with `SharedAppList::set_load_balancer_connectivity` for changed apps only (not `update_app`, which would revert concurrent writers), and broadcast `AppListUpdated` when anything changed
- [x] 6.6 Spawn the watcher from `setup_docker_integration` via `crate::metrics::spawn_instrumented`, only when the load balancer is Traefik and `traefik.watch_docker_events` is true; log once at startup which mode is active
- [x] 6.7 Extend the event filter to `event=["start", "die"]` and update `is_container_start` accordingly (e.g. `is_reconcile_trigger`), so a load balancer going away degrades the reported state within seconds instead of after up to a full `running_app_check`. `die` rather than `stop`, because it also covers `docker kill`, a crash and an OOM; the existing 2s debounce plus single-flight already coalesce the `die`+`start` recreate pair

## 7. Reporting

- [x] 7.1 Add `record_traefik_network_drift_apps` and `record_traefik_network_unroutable_apps` to `metrics/recorder_trait.rs`, implement in `metrics/otel_recorder.rs` (family `scotty_traefik_network_*`, no per-app labels) and `metrics/noop.rs`
- [x] 7.2 Record both gauges at the end of every pass, including zero, and keep a converged pass silent apart from `debug!`. Exception: a pass that could not list networks records nothing, since a zero would be a false all-clear (documented on `record`)

## 8. Wiring the periodic pass

- [x] 8.1 Call `reconcile_traefik_networks` from `docker/setup.rs::schedule_app_check`, inside the `Ok(apps)` arm and *before* `set_apps` so the cache is published already annotated, logging any `Err` without failing the check (being in the `Ok` arm is also what gives "discovery failed → no pruning")
- [x] 8.2 Verify the startup path runs it before the scheduler loop starts (the initial `schedule_app_check` call in `setup_docker_integration`)

## 9. API and clients

- [x] 9.1 Verify `apps/info/{app_id}` and `apps/list` serve the annotated field from the cache (both read `state.apps`), and register `LoadBalancerConnectivity` in the utoipa `components(schemas(...))` list
- [x] 9.2 Add `load_balancer_connectivity: string` to the `App` interface in `frontend/src/types.ts`
- [x] 9.3 Add `frontend/src/components/app-connectivity-pill.svelte` on top of `pill.svelte`: green "Routable", red "Not routable" with an explanatory `title`, amber "LB unavailable", nothing for `NotApplicable`/`Unknown`
- [x] 9.4 Render the pill in the app detail page `PageHeader` `meta` slot next to `<AppStatusPill>` (`routes/dashboard/[slug]/+page.svelte`), and confirm it updates via the existing `apps` store subscription
- [x] 9.5 Add a "Load balancer" row to `format_app_info` in `scottyctl/src/commands/apps/mod.rs`, leaving the `app:list` table columns unchanged
- [x] 9.6 Confirm wire compatibility both ways with unit tests in `connectivity.rs`: a payload without the field deserializes to `Unknown`, unknown extra fields are ignored, and every state round-trips

## 10. Verification

- [x] 10.1 `cargo test` (198 lib + all integration suites) and `cargo clippy --all-targets` clean, `cargo fmt` applied; `bun run check` (0 errors) and `bun run lint` clean
- [x] 10.2 Manual scenario against local Traefik (`apps/traefik`): deploy an app, confirm reachable and the pill reads "Routable", `docker compose up -d --force-recreate traefik`, and confirm the event watcher reconnects the app's `proxy--<app>` network within seconds and the app is reachable again.
      **Ran 2026-07-29, passes at the network level:** with `running_app_check=15m`, `--force-recreate traefik` left it on only `proxy` + `traefik_default` (the #880 outage state); 10s later all of `proxy--simple-nginx`, `proxy--simple-nginx-2`, `proxy--test-env` were reattached and `test-env` reported `Connected`, with one `emitted 'start'` pass logged. HTTP reachability could **not** be confirmed locally: `test-env` 502s because its `.scotty.yml` references `test-middleware`, which is not defined in `apps/traefik/dynamic/` (`middleware "test-middleware@docker" does not exist`), and `simple-nginx` cannot be redeployed here because `docker login` to the placeholder registry fails. Both are local-dev config gaps, unrelated to reconciliation
- [x] 10.3 Manual scenario with `SCOTTY__TRAEFIK__WATCH_DOCKER_EVENTS=false`: same recreate is repaired by the next scheduled `running_app_check` instead, and no events subscription is opened.
      **Ran 2026-07-29, passes:** startup logged `Reconciling proxy networks on every app check only (traefik.watch_docker_events is disabled)`, the recreate was still unrepaired at t+8s and t+16s, the scheduled 1m pass repaired it by t+70s, and zero event-triggered passes were logged
- [ ] 10.4 Manual scenario: block the repair (e.g. remove the app's proxy network while the app runs, or stop the Traefik container) and confirm the detail page shows "Not routable" / "LB unavailable", the error is logged, and `scotty_traefik_unroutable_apps` is non-zero.
      **Ran 2026-07-29 and it failed:** with `traefik` exited, a fresh sweep reported `conn=Connected` for the running `simple-nginx` app, so the pill read "Routable" while nothing routed. Cause: availability was inferred from `inspect_container` succeeding, and a stopped container still lists `proxy--simple-nginx` in `NetworkSettings.Networks`.
      **Re-ran after 3.4 and 6.7, passes:** exited Traefik → `simple-nginx` reports `LoadBalancerUnavailable` with `Traefik container 'traefik' is unavailable ... exists but is not running` logged; `docker compose up -d` → `Connected` within 8s on the `start` event; `docker stop traefik` → `LoadBalancerUnavailable` 6s later on the `die` event, verified with `running_app_check=15m` so no scheduled pass could account for it (log: `Container 'traefik' emitted 'die', reconciling proxy networks`). Still to check by hand: the `scotty_traefik_unroutable_apps` gauge is non-zero, which needs the observability stack up
- [ ] 10.5 Manual scenario: stop an app and confirm it is not attached; remove an app directory by hand and confirm its empty proxy network is pruned while a network with a live foreign container is left alone.
      **Ran 2026-07-29 with two probe networks labelled `scotty.managed=true` (safer than deleting a real app directory, and it exercises the same classify/prune path). Split result:**
      - Guard half **passes**: `proxy--squatted-app`, holding a live foreign container, was left alone — `Skipping orphaned network proxy--squatted-app: still has non-Traefik endpoints attached (foreign-squatter)`
      - Prune half **fails**: `proxy--ghost-app` (orphaned, no endpoints at all) was never removed. `prune_orphans` calls `disconnect_network` unconditionally, and Docker answers `500: container ... is not connected to network proxy--ghost-app`. The tolerance list is `403 | 404 | 409`, so the `continue` fires and `remove_network` is never reached — an orphaned network that Traefik is *not* attached to can never be pruned. Reachable in production: recreate Traefik (which drops its per-app attachments) and then destroy the app, and its network is orphaned with no Traefik endpoint
- [ ] 10.8 Fix the prune path found by 10.5: skip `disconnect_network` when the inspected endpoints do not include the Traefik container (the `attached` vec already answers this), and additionally tolerate `500` on disconnect for the inspect→disconnect race, since `remove_network` is the real gate and reports its own `409` if an endpoint is still attached. Re-run 10.5 afterwards
- [x] 10.6 **Skipped by decision 2026-07-30 — NOT verified.** Manual scenario: restart the Docker daemon and confirm the watcher backs off, resubscribes, and reconciles on resubscribe without the server dying.
      Skipped because a daemon restart takes down every unrelated container on the dev host. Residual risk, all unexercised: the capped 1s→30s backoff never spins against a dead socket; the reconcile-on-(re)subscribe pass recovers a Traefik `start` emitted while the stream was down (without it, apps stay unroutable until the next scheduled sweep); and the watcher task neither panics nor returns, which would silently disable event-driven reconciliation for the process lifetime. Mitigation: all three are accelerator-only paths — the scheduled `running_app_check` still repairs everything, which is what makes skipping tolerable. To run later: start with `running_app_check=15m` so any repair provably came from the resubscribe path, restart Docker, then check the log for `Docker event stream failed, will resubscribe` followed by a reconcile pass, and confirm the `proxy--*` attachments are back
- [x] 10.7 Confirm an app with no public services shows no indicator and is never counted unroutable.
      **Ran 2026-07-29, passes:** `simple-nginx-2` (`Running`, `public_services: []`) reported `NotApplicable` in every pass tonight, including while Traefik was stopped, and was never counted unroutable. `app-connectivity-pill.svelte` computes a null label for `NotApplicable`, so the `{#if label}` guard renders nothing
- [x] 10.9 **Resolved: record the limitation, do not extend detection.** Decide whether connectivity should also verify the *app* side of the proxy network. Observed 2026-07-29: `simple-nginx` and `test-env` both reported `Connected` while returning HTTP 502, because Traefik was attached to `proxy--<app>` but the app's own container was not (legacy override joining the shared `proxy`, or a partially-migrated app). `connectivity_for` only ever checks the load-balancer side, so a half-attached network reads as reachable — which defeats the stated purpose of the indicator ("an app whose containers are running but which is unreachable is visually distinguishable from a healthy one"). The data is already in hand: `inspect_network(...).containers` is fetched for orphan candidates and would answer it. Either extend the requirement or record the limitation explicitly in the spec.
      **Recorded as a limitation:** the connectivity requirement now carries a "Scope limitation — the load-balancer side only" paragraph plus an *Only the load balancer is attached to the proxy network* scenario; the UI requirement's purpose was narrowed from "unreachable" to "load balancer has lost its proxy network membership" and now requires the connected state's explanatory text to describe what was observed (which the existing tooltip already does). Rationale in design Decision 14: verifying the app side costs `1 + n` network reads per pass for a partial answer, `test-env`'s 502 was a missing middleware rather than a missing endpoint so both-sides-attached would still have been green, and reconciliation does not own or repair the app side. Genuine reachability, if wanted, belongs in an explicit health check with its own state. Remaining wart tracked under design Open Questions: the pill label "Routable" still promises more than the state asserts

## 11. Documentation

- [x] 11.1 Document the reconciliation behavior, the new setting, and the connectivity states where per-app proxy networks are described in `docs/`
- [x] 11.2 Update the kenkeep node `.ai/kenkeep/nodes/traefik/map-traefik-per-app-proxy-network.md` to state that Traefik's membership is reconciled on every app check and on Traefik container start, not only at deploy time (via `/kk-add` or a direct node edit)
- [ ] 11.3 Commit with a conventional-commit `fix:` message referencing issue #880, then `openspec archive` the change
