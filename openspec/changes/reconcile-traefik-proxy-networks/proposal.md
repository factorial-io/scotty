## Why

Traefik's membership in each app's per-app proxy network (`<network>--<app>`) is created imperatively at deploy time (`docker network connect`) and lives only in container state — `templates/traefik-compose.yml` declares just the single external `proxy` network. Recreating the Traefik container (stack upgrade, config change, image bump) therefore produces a Traefik that is attached to `proxy` and nothing else, and every previously deployed app becomes unreachable. The failure is silent: containers stay `Up`, health checks pass, TLS still serves the correct certificate, and requests simply hang with no Traefik or app log entry, because the request never reaches a backend. This took down 5 production apps for ~18h (issue #880). Nothing in Scotty repairs the attachment, even though the startup and periodic app check already enumerate every app.

## What Changes

- Add a Traefik proxy-network reconciler that makes Traefik's network membership a **reconciled property** instead of a one-shot deploy side effect. It runs on startup and on every periodic running-app check, right before the refreshed app list is published.
- For every app that has at least one running container, ensure Traefik is connected to that app's existing `<network>--<app>` proxy network; connect it if the attachment is missing. Attachment state is read once per pass from a single Traefik container inspection, so a converged system issues no write calls.
- **Watch Docker events** for the Traefik container starting, and reconcile immediately when it does, so the repair window is seconds instead of up to one `running_app_check` interval. Gated by a new setting `traefik.watch_docker_events`, defaulting to `true`; when disabled, the periodic reconciliation still runs.
- Prune orphaned proxy networks: a Scotty-labelled proxy network (`scotty.managed=true`) whose `scotty.app` no longer exists as a discovered app, and which has no non-Traefik containers attached, is disconnected from Traefik and removed, so membership does not grow unbounded.
- **Expose each app's load-balancer connectivity in the app detail data** (`AppData`), as a distinct state rather than a boolean: connected, disconnected, load balancer unavailable, not applicable, or not yet determined. The reconciler is what fills it in, so the API reports observed Docker state rather than an assumption.
- **Show connectivity in the frontend as an indicator** on the app detail page, next to the app status pill, so a routable app and a silently-unroutable one no longer look identical in the UI. `scottyctl app:info` gains the same information as a row.
- Report the outage condition for operators: log a warning naming each app whose routing was broken and repaired, log an error when an app with public services cannot be made routable (e.g. Traefik container missing), and expose the drift as metrics so it can be alerted on.
- Keep the reconciler a no-op when the load balancer is not Traefik, and leave legacy apps still on the old shared `proxy` network untouched.
- No changes to `app:run`/`app:create` behavior or to the generated compose override. The `AppData` addition is an additive, `serde(default)` field, so it does not break an older `scottyctl` against a newer server or the reverse (see kenkeep `practice-wire-format-changes-must-be-backward-compatible-in-both-directions`).

## Capabilities

### New Capabilities
- `traefik-network-reconciliation`: How Scotty keeps the Traefik container's membership in per-app proxy networks converged with the set of running apps — when reconciliation runs (periodic check and Docker container-start events), what it connects, what it prunes, what it must never touch, and how the resulting connectivity state is reported to operators via logs, metrics, the app detail API, and the web UI.

### Modified Capabilities
<!-- None: no existing spec in openspec/specs/ defines Traefik networking, the app-check loop, or the app detail payload. -->

## Impact

- **scotty**: new `docker/loadbalancer/network_reconciler.rs`; new Docker event watcher task (`docker/setup.rs`, spawned alongside the scheduler); `docker/setup.rs::schedule_app_check` gains the reconcile step; `docker/loadbalancer/mod.rs` gains the shared Traefik-target lookup.
- **scotty-core**: `apps/app_data/data.rs` — `AppData` gains a connectivity field (new enum, `ToSchema` + `serde(default)`); `settings/loadbalancer.rs` — `TraefikSettings` gains `watch_docker_events: bool` (default `true`).
- **Config**: new documented key in `config/default.yaml` and `config/default.yaml.example`; overridable as `SCOTTY__TRAEFIK__WATCH_DOCKER_EVENTS`.
- **Docker API surface**: adds `inspect_container` on the Traefik container, `list_networks` (label-filtered), `inspect_network`, and a long-lived filtered `events` stream, plus the already-used `connect_network`/`disconnect_network`/`remove_network`. Uses the existing bollard 0.21 client on `AppState::docker`; no new dependencies.
- **API / OpenAPI**: `GET /api/v1/authenticated/apps/info/{app_id}` and `apps/list` responses gain the connectivity field; utoipa schema regenerated.
- **frontend**: new connectivity indicator component; app detail page (`routes/dashboard/[slug]/+page.svelte`) renders it; `types.ts` `App` interface gains the field. `bun run check` and `bun run lint` must pass.
- **scottyctl**: `format_app_info` (`commands/apps/mod.rs`) gains a load-balancer row.
- **Metrics**: new gauges in `metrics/recorder_trait.rs`, `metrics/otel_recorder.rs`, `metrics/noop.rs` (new `scotty_traefik_*` metric family).
- **Operational behavior**: on first upgrade after this change, an already-broken host self-heals — within seconds of Traefik starting when event watching is enabled — without manual `docker network connect`; orphaned `proxy--*` networks left by earlier versions are cleaned up.
- **Tests**: unit tests for the desired/actual set diff, orphan classification, and connectivity-state derivation (pure functions, no Docker); documentation of the behavior in `openspec/specs/traefik-network-reconciliation/`.
- **Docs**: the Traefik/networking docs and the kenkeep `traefik` branch node on per-app proxy networks should state that membership is reconciled, not only set at deploy time.
