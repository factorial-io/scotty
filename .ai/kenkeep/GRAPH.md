---
schema_version: 3
nodes_hash: 'sha256:2d5652fdd72103bf2b4f09354d4c063ac4b91fed1d635876e218fed1892ae948'
node_count: 75
---
# kenkeep Graph

Total nodes: 75

## map-app-anatomy

- **kind:** map
- **title:** An app is any folder in the apps directory containing compose.yml
- **path:** map-app-anatomy.md
- **tags:** scotty, apps, vocabulary

## map-app-scope-declaration-in-scotty-yml

- **kind:** map
- **title:** Apps declare authorization scopes in .scotty.yml
- **path:** map-app-scope-declaration-in-scotty-yml.md
- **tags:** authorization, scopes, config

## map-app-types-owned-supported-unsupported

- **kind:** map
- **title:** Apps are categorized as owned, supported, or unsupported
- **path:** map-app-types-owned-supported-unsupported.md
- **tags:** scotty, apps, vocabulary

## map-authorization-assignment-key-format-by-auth-mode

- **kind:** map
- **title:** Casbin policy assignment keys are formatted differently per auth_mode
- **path:** map-authorization-assignment-key-format-by-auth-mode.md
- **tags:** authorization, casbin, auth

## map-authorization-builtin-roles

- **kind:** map
- **title:** Built-in authorization roles and their permission sets
- **path:** map-authorization-builtin-roles.md
- **tags:** authorization, rbac, roles, casbin

## map-authorization-casbin-rbac

- **kind:** map
- **title:** Authorization uses Casbin RBAC
- **path:** map-authorization-casbin-rbac.md
- **tags:** authorization, casbin, security, map

## map-blueprint-injected-env-vars

- **kind:** map
- **title:** Blueprint action scripts get SCOTTY__APP_NAME and SCOTTY__PUBLIC_URL__<SERVICE> injected
- **path:** map-blueprint-injected-env-vars.md
- **tags:** blueprints, configuration, env

## map-blueprints-concept

- **kind:** map
- **title:** Blueprints are reusable app templates
- **path:** map-blueprints-concept.md
- **tags:** blueprints, map

## map-cli-app-cp-permission-and-size-limit

- **kind:** map
- **title:** app:cp permission split and transfer size limit
- **path:** map-cli-app-cp-permission-and-size-limit.md
- **tags:** cli, app-cp, permissions, authorization

## map-cli-app-create-registry-middleware-allowlist

- **kind:** map
- **title:** app:create --registry and --middleware require server-side allow-listing
- **path:** map-cli-app-create-registry-middleware-allowlist.md
- **tags:** cli, app-create, traefik, middleware

## map-cli-app-create-robots-header-default

- **kind:** map
- **title:** app:create injects a noindex X-Robots-Tag by default
- **path:** map-cli-app-create-robots-header-default.md
- **tags:** cli, app-create, traefik, seo

## map-config-directory-git-tracking

- **kind:** map
- **title:** Which files under config/ are committed vs git-ignored
- **path:** map-config-directory-git-tracking.md
- **tags:** config, casbin, git, structure

## map-container-log-streaming-for-stopped-containers

- **kind:** map
- **title:** Log streaming behavior for stopped vs missing containers
- **path:** map-container-log-streaming-for-stopped-containers.md
- **tags:** logs, docker, websocket

## map-custom-action-execution-lookup-and-gate

- **kind:** map
- **title:** Custom action execution checks per-app actions before blueprint actions and always gates on can_execute()
- **path:** map-custom-action-execution-lookup-and-gate.md
- **tags:** custom-actions, authorization, blueprints

## map-custom-actions-approval-workflow

- **kind:** map
- **title:** Custom actions require approval before execution
- **path:** map-custom-actions-approval-workflow.md
- **tags:** custom-actions, workflow, security

## map-default-backend-landing-page

- **kind:** map
- **title:** Scotty as Traefik default backend / landing page
- **path:** map-default-backend-landing-page.md
- **tags:** landing-page, traefik, load-balancer, architecture

## map-default-backend-security-model

- **kind:** map
- **title:** Default-backend landing page security properties
- **path:** map-default-backend-security-model.md
- **tags:** security, landing-page, authorization

## map-docker-image-excludes-secrets

- **kind:** map
- **title:** Scotty Docker image ships binaries and non-sensitive config only
- **path:** map-docker-image-excludes-secrets.md
- **tags:** docker, deployment, config, security

## map-follow-mode-unavailable-for-stopped-containers

- **kind:** map
- **title:** Follow mode is a no-op notice, not an error, on stopped containers
- **path:** map-follow-mode-unavailable-for-stopped-containers.md
- **tags:** logs, docker, websocket, frontend

## map-frontend-src-layout

- **kind:** map
- **title:** Frontend src/ layout and dev-server proxy targets
- **path:** map-frontend-src-layout.md
- **tags:** frontend, structure, sveltekit

## map-load-balancer-support

- **kind:** map
- **title:** Scotty supports Traefik and legacy Haproxy-config load balancers
- **path:** map-load-balancer-support.md
- **tags:** scotty, traefik, haproxy, load-balancer

## map-oauth-route-protection-split

- **kind:** map
- **title:** Public vs protected routes under OAuth mode
- **path:** map-oauth-route-protection-split.md
- **tags:** oauth, auth, routing, security

## map-oauth-secret-storage-and-session-lifetimes

- **kind:** map
- **title:** OAuth PKCE/CSRF secrets use MaskedSecret; session lifetimes are short-lived
- **path:** map-oauth-secret-storage-and-session-lifetimes.md
- **tags:** oauth, security, sessions

## map-observability-config-files

- **kind:** map
- **title:** Observability stack config file locations
- **path:** map-observability-config-files.md
- **tags:** observability, grafana, configuration

## map-observability-data-retention

- **kind:** map
- **title:** Observability stack data retention limits
- **path:** map-observability-data-retention.md
- **tags:** observability, jaeger, victoriametrics

## map-observability-local-access

- **kind:** map
- **title:** Local observability stack: prerequisite and access URLs
- **path:** map-observability-local-access.md
- **tags:** observability, ddev, traefik, local-dev

## map-observability-stack-architecture

- **kind:** map
- **title:** Observability stack architecture
- **path:** map-observability-stack-architecture.md
- **tags:** observability, opentelemetry, metrics, tracing, grafana

## map-onepassword-secret-resolution

- **kind:** map
- **title:** 1Password secrets are resolved via op:// URIs in app env vars only
- **path:** map-onepassword-secret-resolution.md
- **tags:** 1password, secrets, configuration

## map-pre-push-hook-cargo-husky

- **kind:** map
- **title:** Pre-push git hook installed via cargo-husky
- **path:** map-pre-push-hook-cargo-husky.md
- **tags:** git, tooling, hooks, husky

## map-project-workspace-components

- **kind:** map
- **title:** Scotty workspace components
- **path:** map-project-workspace-components.md
- **tags:** architecture, workspace, overview

## map-rate-limiting-tiers

- **kind:** map
- **title:** Rate limiting has three independent tiers keyed differently
- **path:** map-rate-limiting-tiers.md
- **tags:** rate-limiting, security, api

## map-rustls-crypto-provider-init

- **kind:** map
- **title:** rustls CryptoProvider is installed explicitly at process start
- **path:** map-rustls-crypto-provider-init.md
- **tags:** tls, rustls, startup, docker-build

## map-scotty-metrics-families

- **kind:** map
- **title:** Scotty metrics families and prefix
- **path:** map-scotty-metrics-families.md
- **tags:** observability, metrics

## map-scotty-product-scope

- **kind:** map
- **title:** Scotty's scope: single-node micro-PaaS, not a cluster orchestrator
- **path:** map-scotty-product-scope.md
- **tags:** architecture, scope, positioning

## map-scotty-server-module-map

- **kind:** map
- **title:** Scotty server key modules and their locations
- **path:** map-scotty-server-module-map.md
- **tags:** architecture, server, map

## map-scottyctl-app-cp

- **kind:** map
- **title:** scottyctl app:cp moves files between workstation and app containers
- **path:** map-scottyctl-app-cp.md
- **tags:** scottyctl, cli, docker

## map-scottyctl-cli-structure

- **kind:** map
- **title:** scottyctl CLI namespace and behavior
- **path:** map-scottyctl-cli-structure.md
- **tags:** cli, scottyctl, map

## map-state-machine-errors-bubble-to-task-failure

- **kind:** map
- **title:** State machine handler errors always mark the task Failed
- **path:** map-state-machine-errors-bubble-to-task-failure.md
- **tags:** state-machine, tasks, docker

## map-traefik-per-app-proxy-network

- **kind:** map
- **title:** Each app gets its own dedicated Traefik proxy network
- **path:** map-traefik-per-app-proxy-network.md
- **tags:** traefik, docker, networking, loadbalancer

## practice-access-token-config-removed-use-bearer-tokens

- **kind:** practice
- **title:** api.access_token is no longer supported — use api.bearer_tokens
- **path:** practice-access-token-config-removed-use-bearer-tokens.md
- **tags:** auth, configuration, gotcha

## practice-adopt-then-rebuild-for-unsupported-apps

- **kind:** practice
- **title:** Run app:rebuild after app:adopt to actually enable load balancing
- **path:** practice-adopt-then-rebuild-for-unsupported-apps.md
- **tags:** scotty, cli, workflow, gotcha

## practice-app-cp-path-spec-and-pipe-mode

- **kind:** practice
- **title:** app:cp path-spec parsing and pipe-mode semantics
- **path:** practice-app-cp-path-spec-and-pipe-mode.md
- **tags:** file-transfer, cli, scottyctl

## practice-bearer-token-constant-time-comparison

- **kind:** practice
- **title:** Bearer token comparison is constant-time
- **path:** practice-bearer-token-constant-time-comparison.md
- **tags:** security, auth, bearer-token

## practice-bearer-token-identifier-naming

- **kind:** practice
- **title:** Bearer token identifiers vs secret values; identifier: prefix for service accounts
- **path:** practice-bearer-token-identifier-naming.md
- **tags:** auth, casbin, bearer-tokens, naming

## practice-bearer-tokens-require-rbac-assignment

- **kind:** practice
- **title:** Bearer tokens must be explicitly assigned in authorization policy
- **path:** practice-bearer-tokens-require-rbac-assignment.md
- **tags:** authorization, bearer-token, casbin, auth

## practice-casbin-assignment-precedence

- **kind:** practice
- **title:** Casbin assignment matching follows exact > domain > wildcard precedence
- **path:** practice-casbin-assignment-precedence.md
- **tags:** authorization, casbin, security

## practice-cli-app-purge-vs-destroy

- **kind:** practice
- **title:** app:purge only clears ephemeral data; app:destroy is the irreversible one
- **path:** practice-cli-app-purge-vs-destroy.md
- **tags:** cli, app-purge, app-destroy, gotcha

## practice-cli-app-shell-security-characteristics

- **kind:** practice
- **title:** app:shell security characteristics to account for
- **path:** practice-cli-app-shell-security-characteristics.md
- **tags:** cli, app-shell, security

## practice-config-env-var-override-convention

- **kind:** practice
- **title:** Config keys are overridden via SCOTTY__ prefixed, double-underscore env vars
- **path:** practice-config-env-var-override-convention.md
- **tags:** configuration, env

## practice-container-status-one-shot-completion

- **kind:** practice
- **title:** Running status treats clean one-shot exits as completed, gates URLs per-service
- **path:** practice-container-status-one-shot-completion.md
- **tags:** docker, status, container, frontend

## practice-default-backend-configuration

- **kind:** practice
- **title:** Default-backend setup requires api.base_url and a low-priority catch-all router
- **path:** practice-default-backend-configuration.md
- **tags:** traefik, configuration, landing-page, gotcha

## practice-dotenv-precedence-scotty

- **kind:** practice
- **title:** .env / .env.local precedence and usage convention
- **path:** practice-dotenv-precedence-scotty.md
- **tags:** configuration, env, local-development

## practice-enable-telemetry-env-var

- **kind:** practice
- **title:** Enable metrics/traces export via SCOTTY__TELEMETRY
- **path:** practice-enable-telemetry-env-var.md
- **tags:** observability, config, env-vars

## practice-file-transfer-http-tar-streaming

- **kind:** practice
- **title:** File transfers stream over HTTP chunked tar, not WebSocket
- **path:** practice-file-transfer-http-tar-streaming.md
- **tags:** file-transfer, docker, streaming, http

## practice-frontend-backend-tight-coupling

- **kind:** practice
- **title:** Frontend has no API versioning, evolves in lockstep with backend
- **path:** practice-frontend-backend-tight-coupling.md
- **tags:** frontend, api, architecture

## practice-frontend-types-regenerate-after-backend-change

- **kind:** practice
- **title:** Regenerate frontend TypeScript types after backend Rust type changes
- **path:** practice-frontend-types-regenerate-after-backend-change.md
- **tags:** frontend, types, ts-rs, workflow

## practice-frontend-uses-bun

- **kind:** practice
- **title:** Frontend tooling uses bun, not npm
- **path:** practice-frontend-uses-bun.md
- **tags:** frontend, tooling, bun

## practice-git-rules

- **kind:** practice
- **title:** Git conventions for this repo
- **path:** practice-git-rules.md
- **tags:** git, conventions

## practice-hybrid-auth-bearer-checked-first

- **kind:** practice
- **title:** In hybrid OAuth+bearer mode, bearer tokens are checked before OAuth
- **path:** practice-hybrid-auth-bearer-checked-first.md
- **tags:** oauth, bearer-tokens, auth, performance

## practice-local-dev-traefik-prereq

- **kind:** practice
- **title:** Start Traefik before local development
- **path:** practice-local-dev-traefik-prereq.md
- **tags:** local-dev, traefik, prerequisites

## practice-localhost-subdomain-dns-gotcha

- **kind:** practice
- **title:** Local *.localhost subdomains may not auto-resolve to 127.0.0.1
- **path:** practice-localhost-subdomain-dns-gotcha.md
- **tags:** local-dev, dns, traefik, gotcha

## practice-never-store-real-bearer-tokens-in-config

- **kind:** practice
- **title:** Never store real bearer tokens in configuration files
- **path:** practice-never-store-real-bearer-tokens-in-config.md
- **tags:** security, auth, configuration, secrets

## practice-oauth-redirect-url-vs-frontend-base-url

- **kind:** practice
- **title:** OAuth config has two distinct URLs that must not be confused
- **path:** practice-oauth-redirect-url-vs-frontend-base-url.md
- **tags:** oauth, configuration, gotcha

## practice-observability-stack-swappable

- **kind:** practice
- **title:** Observability backends are swappable via open standards
- **path:** practice-observability-stack-swappable.md
- **tags:** observability, prometheus, architecture

## practice-project-management-beans

- **kind:** practice
- **title:** Use beans CLI for issue tracking, not ad hoc todo lists
- **path:** practice-project-management-beans.md
- **tags:** project-management, beans, workflow

## practice-rate-limiting-is-per-instance-not-global

- **kind:** practice
- **title:** Rate limits are per-instance, not global, across multiple Scotty instances
- **path:** practice-rate-limiting-is-per-instance-not-global.md
- **tags:** rate-limiting, deployment, gotcha

## practice-release-process-automation

- **kind:** practice
- **title:** Releases are fully automated via release-please
- **path:** practice-release-process-automation.md
- **tags:** release, versioning, ci

## practice-root-folder-must-match-docker-mount-path

- **kind:** practice
- **title:** apps.root_folder must match the host mount path when Scotty runs in Docker
- **path:** practice-root-folder-must-match-docker-mount-path.md
- **tags:** docker, configuration, gotcha

## practice-scotty-only-touches-override-and-settings-files

- **kind:** practice
- **title:** Scotty only writes compose.override.yml and .scotty.yml in an app directory
- **path:** practice-scotty-only-touches-override-and-settings-files.md
- **tags:** scotty, compose, file-ownership

## practice-scottyctl-access-token-precedence

- **kind:** practice
- **title:** scottyctl: explicit access token wins over cached OAuth token
- **path:** practice-scottyctl-access-token-precedence.md
- **tags:** scottyctl, auth, oauth, cli

## practice-settings-config-precedence

- **kind:** practice
- **title:** Settings load order and secret handling
- **path:** practice-settings-config-precedence.md
- **tags:** configuration, settings, security

## practice-testing-conventions

- **kind:** practice
- **title:** Test placement and tooling conventions
- **path:** practice-testing-conventions.md
- **tags:** testing, conventions

## practice-traefik-middleware-ordering

- **kind:** practice
- **title:** Traefik middlewares: built-ins apply before custom, order matters
- **path:** practice-traefik-middleware-ordering.md
- **tags:** traefik, middleware, scotty-yml, loadbalancer

## practice-traefik-network-migration-requires-rebuild

- **kind:** practice
- **title:** Migrating an app from the old shared Traefik network requires app:rebuild
- **path:** practice-traefik-network-migration-requires-rebuild.md
- **tags:** traefik, docker, networking, migration

## practice-unsupported-compose-features

- **kind:** practice
- **title:** compose.yml must not expose ports directly or use env-var expansion
- **path:** practice-unsupported-compose-features.md
- **tags:** scotty, compose, restriction
