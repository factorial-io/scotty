# kenkeep Index: observability

↑ Parent: [kenkeep](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
- Open [**Observability backends are swappable via open standards**](practice-observability-stack-swappable.md) to learn about: Scotty's telemetry uses OTLP, PromQL, and W3C Trace Context so any component (VictoriaMetrics, Jaeger, Grafana) can be replaced with a compatible alternative. #observability #prometheus #architecture

## Components (what exists)
- Open [**Local observability stack: prerequisite and access URLs**](map-observability-local-access.md) to learn about: The observability stack (observability/docker-compose) needs Traefik running first for .ddev.site routing; Grafana/Jaeger/VictoriaMetrics are reached via *.ddev.site URLs. #observability #ddev #traefik #local-dev
- Open [**Observability stack architecture**](map-observability-stack-architecture.md) to learn about: Scotty exports OTLP telemetry to an OpenTelemetry Collector, which routes traces to Jaeger and metrics to VictoriaMetrics, visualized in Grafana. #observability #opentelemetry #metrics #tracing #grafana
- Open [**Observability stack config file locations**](map-observability-config-files.md) to learn about: docker-compose.yml, otel-collector-config.yaml, and grafana/ provisioning/dashboards dirs define the observability stack's setup. #observability #grafana #configuration
- Open [**Observability stack data retention limits**](map-observability-data-retention.md) to learn about: VictoriaMetrics retains metrics 30 days by default (configurable); Jaeger traces are in-memory only and lost on restart. #observability #jaeger #victoriametrics
- Open [**Scotty metrics families and prefix**](map-scotty-metrics-families.md) to learn about: All Scotty metrics use the scotty_ prefix, grouped by subsystem: log streaming, shell sessions, websocket, tasks, HTTP server, memory, application fleet, and Tokio runtime. #observability #metrics

## By topic

### #observability
- Open [**Scotty metrics families and prefix**](map-scotty-metrics-families.md) — All Scotty metrics use the scotty_ prefix, grouped by subsystem: log streaming, shell sessions, websocket, tasks, HTTP server, memory, application fleet, and Tokio runtime.
- Open [**Observability stack config file locations**](map-observability-config-files.md) — docker-compose.yml, otel-collector-config.yaml, and grafana/ provisioning/dashboards dirs define the observability stack's setup.
- Open [**Observability stack architecture**](map-observability-stack-architecture.md) — Scotty exports OTLP telemetry to an OpenTelemetry Collector, which routes traces to Jaeger and metrics to VictoriaMetrics, visualized in Grafana.
### #grafana
- Open [**Observability stack architecture**](map-observability-stack-architecture.md) — Scotty exports OTLP telemetry to an OpenTelemetry Collector, which routes traces to Jaeger and metrics to VictoriaMetrics, visualized in Grafana.
- Open [**Observability stack config file locations**](map-observability-config-files.md) — docker-compose.yml, otel-collector-config.yaml, and grafana/ provisioning/dashboards dirs define the observability stack's setup.
### #metrics
- Open [**Observability stack architecture**](map-observability-stack-architecture.md) — Scotty exports OTLP telemetry to an OpenTelemetry Collector, which routes traces to Jaeger and metrics to VictoriaMetrics, visualized in Grafana.
- Open [**Scotty metrics families and prefix**](map-scotty-metrics-families.md) — All Scotty metrics use the scotty_ prefix, grouped by subsystem: log streaming, shell sessions, websocket, tasks, HTTP server, memory, application fleet, and Tokio runtime.
### #architecture
- Open [**Frontend has no API versioning, evolves in lockstep with backend**](../frontend/practice-frontend-backend-tight-coupling.md) — Scotty frontend is tightly coupled to the backend API; no versioning or backwards compatibility is maintained, so breaking API changes are acceptable.
- Open [**Observability backends are swappable via open standards**](practice-observability-stack-swappable.md) — Scotty's telemetry uses OTLP, PromQL, and W3C Trace Context so any component (VictoriaMetrics, Jaeger, Grafana) can be replaced with a compatible alternative.
- Open [**Scotty server key modules and their locations**](../architecture/map-scotty-server-module-map.md) — Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks, notification, metrics) to responsibilities.
### #configuration
- Open [**Config keys are overridden via SCOTTY__ prefixed, double-underscore env vars**](../configuration/practice-config-env-var-override-convention.md) — Any config.yaml key can be overridden by an env var: prefix SCOTTY__, replace dots/nesting with double underscores.
- Open [**api.access_token is legacy — only honored in the Casbin fallback path**](../configuration/practice-access-token-config-removed-use-bearer-tokens.md) — api.access_token still exists but is only used when the Casbin config fails to load, where it grants admin on the default scope; use api.bearer_tokens.
- Open [**apps.root_folder must match the host mount path when Scotty runs in Docker**](../configuration/practice-root-folder-must-match-docker-mount-path.md) — If Scotty runs containerized, the apps root_folder path inside the container must equal the host path, or docker-compose fails to run apps.
### #ddev
- Open [**Local observability stack: prerequisite and access URLs**](map-observability-local-access.md) — The observability stack (observability/docker-compose) needs Traefik running first for .ddev.site routing; Grafana/Jaeger/VictoriaMetrics are reached via *.ddev.site URLs.
### #jaeger
- Open [**Observability stack data retention limits**](map-observability-data-retention.md) — VictoriaMetrics retains metrics 30 days by default (configurable); Jaeger traces are in-memory only and lost on restart.
### #local-dev
- Open [**Start Traefik before local development**](../traefik/practice-local-dev-traefik-prereq.md) — Local dev requires Traefik running via docker compose in apps/traefik as a prerequisite.
- Open [**Local *.localhost subdomains may not auto-resolve to 127.0.0.1**](../traefik/practice-localhost-subdomain-dns-gotcha.md) — Not all systems resolve wildcard *.localhost subdomains; add explicit /etc/hosts entries for each app hostname used in local dev.
- Open [**Local observability stack: prerequisite and access URLs**](map-observability-local-access.md) — The observability stack (observability/docker-compose) needs Traefik running first for .ddev.site routing; Grafana/Jaeger/VictoriaMetrics are reached via *.ddev.site URLs.
### #opentelemetry
- Open [**Observability stack architecture**](map-observability-stack-architecture.md) — Scotty exports OTLP telemetry to an OpenTelemetry Collector, which routes traces to Jaeger and metrics to VictoriaMetrics, visualized in Grafana.
### #prometheus
- Open [**Observability backends are swappable via open standards**](practice-observability-stack-swappable.md) — Scotty's telemetry uses OTLP, PromQL, and W3C Trace Context so any component (VictoriaMetrics, Jaeger, Grafana) can be replaced with a compatible alternative.
### #tracing
- Open [**Observability stack architecture**](map-observability-stack-architecture.md) — Scotty exports OTLP telemetry to an OpenTelemetry Collector, which routes traces to Jaeger and metrics to VictoriaMetrics, visualized in Grafana.
### #traefik
- Open [**Start Traefik before local development**](../traefik/practice-local-dev-traefik-prereq.md) — Local dev requires Traefik running via docker compose in apps/traefik as a prerequisite.
- Open [**app:create --registry and --middleware require server-side allow-listing**](../cli/map-cli-app-create-registry-middleware-allowlist.md) — Custom registries and middleware referenced by app:create must be pre-configured/allow-listed on the server; middleware is Traefik-only.
- Open [**Each app gets its own dedicated Traefik proxy network**](../traefik/map-traefik-per-app-proxy-network.md) — Scotty creates a per-app network (<network>--<app-name>) instead of one shared network, to avoid Docker DNS alias collisions.
### #victoriametrics
- Open [**Observability stack data retention limits**](map-observability-data-retention.md) — VictoriaMetrics retains metrics 30 days by default (configurable); Jaeger traces are in-memory only and lost on restart.