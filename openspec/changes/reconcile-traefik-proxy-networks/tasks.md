## 1. Reconciler scaffolding

- [ ] 1.1 Add `scotty/src/docker/loadbalancer/network_reconciler.rs` and register it in `loadbalancer/mod.rs`; define the entry point `reconcile_traefik_networks(app_state: &SharedAppState, apps: &AppDataVec) -> anyhow::Result<()>` that returns `Ok(())` immediately when `settings.load_balancer_type != LoadBalancerType::Traefik`
- [ ] 1.2 Move/share the "is Traefik + resolve container name + base network" lookup so the reconciler and `state_machine_handlers/network_handler.rs::proxy_network_target` cannot drift apart
- [ ] 1.3 Confirm the bollard 0.21 signatures for `list_networks` (label filter) and `inspect_network` against the crate, and add the label-filter helper (`label=scotty.managed=true`)

## 2. Observation

- [ ] 2.1 Read Traefik's current membership: `inspect_container(container_name, None::<InspectContainerOptions>)` → `NetworkSettings.Networks` keys as a `HashSet<String>`; on failure log `error!` once, count apps with public services as unroutable, and return `Ok(())`
- [ ] 2.2 List candidate networks via label-filtered `list_networks`, capturing each network's name and its `scotty.app` label; on failure log `error!` and skip the pass
- [ ] 2.3 Add the name-based fallback set: for each discovered app, `app_proxy_network_name(base, &app.name)` matched against existing network names, so unlabelled per-app networks are still connect-eligible (never prune-eligible)

## 3. Classification (pure, unit-testable)

- [ ] 3.1 Implement `classify(...) -> NetworkVerdict` with `Desired` (app discovered and has a container where `ContainerState::is_running()`), `Ignore` (app discovered, nothing running), `OrphanCandidate` (no matching discovered app)
- [ ] 3.2 Implement `is_prunable(inspected_containers, traefik_name) -> bool`: true only when no attached endpoint other than the Traefik container remains
- [ ] 3.3 Implement the "has public services" predicate from `app.settings.public_services` (apps without settings are reconciled but never counted unroutable)
- [ ] 3.4 Unit-test 3.1–3.3 with hand-built `AppDataVec`/network fixtures: running app, stopped app, unknown app, legacy app with no per-app network, unlabelled per-app network, base network itself, network with a foreign container attached

## 4. Convergence actions

- [ ] 4.1 Connect: for each `Desired` network absent from Traefik's membership, `warn!` naming app and network, `connect_network`, `info!` on success, tolerating 403/409 (already connected) and 404 (network/container vanished mid-pass) exactly as `EnsureAppNetworkHandler` does
- [ ] 4.2 Prune: for each `OrphanCandidate`, `inspect_network`, and when `is_prunable`, `disconnect_network` with `force` then `remove_network`, tolerating 403/404/409; `info!` each prune and each skipped orphan with the reason
- [ ] 4.3 Ensure no single-app failure aborts the pass: accumulate failures, keep iterating, and never create a network in this code path

## 5. Reporting

- [ ] 5.1 Add `record_traefik_network_drift_apps` and `record_traefik_unroutable_apps` to `metrics/recorder_trait.rs`, implement in `metrics/otel_recorder.rs` (family `scotty_traefik_*`, no per-app labels) and `metrics/noop.rs`
- [ ] 5.2 Record both gauges at the end of every pass, including zero, and keep a converged pass silent apart from `debug!`

## 6. Wiring

- [ ] 6.1 Call `reconcile_traefik_networks` from `docker/setup.rs::schedule_app_check`, inside the `Ok(apps)` arm after `set_apps`, logging any `Err` without failing the check (this is also what gives "discovery failed → no pruning")
- [ ] 6.2 Verify the startup path runs it before the scheduler loop starts (the initial `schedule_app_check` call in `setup_docker_integration`)

## 7. Verification

- [ ] 7.1 `cargo test` and `cargo clippy --all-targets` clean
- [ ] 7.2 Manual scenario against local Traefik (`apps/traefik`): deploy an app, confirm reachable, `docker compose up -d --force-recreate traefik`, confirm the hostname hangs and `docker inspect traefik` shows only `proxy`, then confirm the next app check reconnects the app's `proxy--<app>` network and the app is reachable again
- [ ] 7.3 Manual scenario: stop an app and confirm it is not attached; remove an app directory by hand and confirm its empty proxy network is pruned while a network with a live foreign container is left alone
- [ ] 7.4 Manual scenario: stop the Traefik container entirely and confirm the app check still completes, logs the unroutable error, and reports a non-zero `scotty_traefik_unroutable_apps`

## 8. Documentation

- [ ] 8.1 Document the reconciliation behavior where per-app proxy networks are described in `docs/`
- [ ] 8.2 Update the kenkeep node `.ai/kenkeep/nodes/traefik/map-traefik-per-app-proxy-network.md` to state that Traefik's membership is reconciled on every app check, not only at deploy time (via `/kk-add` or a direct node edit)
- [ ] 8.3 Commit with a conventional-commit `fix:` message referencing issue #880, then `openspec archive` the change
