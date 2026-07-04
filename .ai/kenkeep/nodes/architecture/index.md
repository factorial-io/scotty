# kenkeep Index: architecture

↑ Parent: [kenkeep](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
_None yet._

## Components (what exists)
- Open [**rustls CryptoProvider is installed explicitly at process start**](map-rustls-crypto-provider-init.md) to learn about: scotty and scottyctl both call scotty_core::http::ensure_crypto_provider() before any TLS use, and Docker builds pin dependency versions with --locked. #tls #rustls #startup #docker-build
- Open [**Scotty server key modules and their locations**](map-scotty-server-module-map.md) to learn about: Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks, notification, metrics) to responsibilities. #architecture #server #map
- Open [**Scotty workspace components**](map-project-workspace-components.md) to learn about: The crates/apps making up the Scotty micro-PaaS and what each one does. #architecture #workspace #overview
- Open [**Scotty's scope: single-node micro-PaaS, not a cluster orchestrator**](map-scotty-product-scope.md) to learn about: Scotty is a single-node docker-compose orchestrator for ephemeral review apps, not a Kubernetes/Nomad replacement. #architecture #scope #positioning

## By topic

### #architecture
- Open [**Frontend has no API versioning, evolves in lockstep with backend**](../frontend/practice-frontend-backend-tight-coupling.md) — Scotty frontend is tightly coupled to the backend API; no versioning or backwards compatibility is maintained, so breaking API changes are acceptable.
- Open [**Observability backends are swappable via open standards**](../observability/practice-observability-stack-swappable.md) — Scotty's telemetry uses OTLP, PromQL, and W3C Trace Context so any component (VictoriaMetrics, Jaeger, Grafana) can be replaced with a compatible alternative.
- Open [**Scotty server key modules and their locations**](map-scotty-server-module-map.md) — Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks, notification, metrics) to responsibilities.
### #docker-build
- Open [**rustls CryptoProvider is installed explicitly at process start**](map-rustls-crypto-provider-init.md) — scotty and scottyctl both call scotty_core::http::ensure_crypto_provider() before any TLS use, and Docker builds pin dependency versions with --locked.
### #map
- Open [**Blueprints are reusable app templates**](../apps/map-blueprints-concept.md) — Templates defining required/public services, port mappings, lifecycle actions, and per-service custom actions.
- Open [**Scotty server key modules and their locations**](map-scotty-server-module-map.md) — Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks, notification, metrics) to responsibilities.
- Open [**scottyctl CLI namespace and behavior**](../cli/map-scottyctl-cli-structure.md) — Colon-namespaced commands, global flags, version preflight check, and gzip+base64 file upload with .scottyignore.
### #overview
- Open [**Scotty workspace components**](map-project-workspace-components.md) — The crates/apps making up the Scotty micro-PaaS and what each one does.
### #positioning
- Open [**Scotty's scope: single-node micro-PaaS, not a cluster orchestrator**](map-scotty-product-scope.md) — Scotty is a single-node docker-compose orchestrator for ephemeral review apps, not a Kubernetes/Nomad replacement.
### #rustls
- Open [**rustls CryptoProvider is installed explicitly at process start**](map-rustls-crypto-provider-init.md) — scotty and scottyctl both call scotty_core::http::ensure_crypto_provider() before any TLS use, and Docker builds pin dependency versions with --locked.
### #scope
- Open [**Scotty's scope: single-node micro-PaaS, not a cluster orchestrator**](map-scotty-product-scope.md) — Scotty is a single-node docker-compose orchestrator for ephemeral review apps, not a Kubernetes/Nomad replacement.
### #server
- Open [**Scotty server key modules and their locations**](map-scotty-server-module-map.md) — Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks, notification, metrics) to responsibilities.
### #startup
- Open [**rustls CryptoProvider is installed explicitly at process start**](map-rustls-crypto-provider-init.md) — scotty and scottyctl both call scotty_core::http::ensure_crypto_provider() before any TLS use, and Docker builds pin dependency versions with --locked.
### #tls
- Open [**rustls CryptoProvider is installed explicitly at process start**](map-rustls-crypto-provider-init.md) — scotty and scottyctl both call scotty_core::http::ensure_crypto_provider() before any TLS use, and Docker builds pin dependency versions with --locked.
### #workspace
- Open [**Scotty workspace components**](map-project-workspace-components.md) — The crates/apps making up the Scotty micro-PaaS and what each one does.