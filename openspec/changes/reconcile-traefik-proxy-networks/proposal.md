## Why

Traefik's membership in each app's per-app proxy network (`<network>--<app>`) is created imperatively at deploy time (`docker network connect`) and lives only in container state — `templates/traefik-compose.yml` declares just the single external `proxy` network. Recreating the Traefik container (stack upgrade, config change, image bump) therefore produces a Traefik that is attached to `proxy` and nothing else, and every previously deployed app becomes unreachable. The failure is silent: containers stay `Up`, health checks pass, TLS still serves the correct certificate, and requests simply hang with no Traefik or app log entry, because the request never reaches a backend. This took down 5 production apps for ~18h (issue #880). Nothing in Scotty repairs the attachment, even though the startup and periodic app check already enumerate every app.

## What Changes

- Add a Traefik proxy-network reconciler that makes Traefik's network membership a **reconciled property** instead of a one-shot deploy side effect. It runs on startup and on every periodic running-app check, right after the app list is refreshed.
- For every app that has at least one running container, ensure Traefik is connected to that app's existing `<network>--<app>` proxy network; connect it if the attachment is missing. Attachment state is read once per pass from a single Traefik container inspection, so a converged system issues no write calls.
- Prune orphaned proxy networks: a Scotty-labelled proxy network (`scotty.managed=true`) whose `scotty.app` no longer exists as a discovered app, and which has no non-Traefik containers attached, is disconnected from Traefik and removed, so membership does not grow unbounded.
- Report the outage condition explicitly: log a warning naming each app whose routing was broken and repaired, log an error when an app with public services cannot be made routable (e.g. Traefik container missing), and expose the drift as metrics so it is visible in Grafana rather than only after user reports.
- Keep the reconciler a no-op when the load balancer is not Traefik, and leave legacy apps still on the old shared `proxy` network untouched.
- No changes to `app:run`/`app:create` behavior, to the generated compose override, or to any API contract; reconciliation is purely additive and idempotent.

## Capabilities

### New Capabilities
- `traefik-network-reconciliation`: How Scotty keeps the Traefik container's membership in per-app proxy networks converged with the set of running apps — when reconciliation runs, what it connects, what it prunes, what it must never touch, and how drift is reported to operators.

### Modified Capabilities
<!-- None: no existing spec in openspec/specs/ defines Traefik networking or the app-check loop. -->

## Impact

- **scotty**: new `docker/traefik_network_reconciler.rs` (or equivalent module under `docker/loadbalancer/`); `docker/setup.rs::schedule_app_check` gains the reconcile step after `find_apps`; `docker/loadbalancer/mod.rs` may gain a helper to parse an app name out of a proxy-network name.
- **Docker API surface**: adds `inspect_container` on the Traefik container, `list_networks` (label-filtered), `inspect_network`, plus the already-used `connect_network`/`disconnect_network`/`remove_network`. Uses existing bollard 0.21 client on `AppState::docker`; no new dependencies.
- **Metrics**: new gauges/counters in `metrics/recorder_trait.rs`, `metrics/otel_recorder.rs`, `metrics/noop.rs`, plus a sampling point in the reconciler (new `scotty_traefik_*` metric family).
- **Operational behavior**: on first upgrade after this change, an already-broken host self-heals on the next app check without manual `docker network connect`; orphaned `proxy--*` networks left by earlier versions are cleaned up.
- **Tests**: unit tests for the desired/actual set diff and orphan-classification logic (pure functions, no Docker); documentation of the behavior in `openspec/specs/traefik-network-reconciliation/`.
- **Docs**: the Traefik/networking docs and the kenkeep `traefik` branch node on per-app proxy networks should mention that membership is reconciled, not only set at deploy time.
