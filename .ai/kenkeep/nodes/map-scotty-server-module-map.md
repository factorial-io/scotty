---
type: map
title: Scotty server key modules and their locations
description: >-
  Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks,
  notification, metrics) to responsibilities.
tags:
  - architecture
  - server
  - map
kk_schema_version: 3
kk_id: map-scotty-server-module-map
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
Entry point is `main.rs`, which initializes AppState (settings, Docker client, task manager), sets up OpenTelemetry, and spawns the HTTP server and background tasks.

Key modules under `scotty/src/`: `api/router.rs` (Axum router with OpenAPI/utoipa docs); `api/rest/handlers/` (REST endpoints for apps, admin, scopes, blueprints, landing/Traefik fallback, login, tasks, health, info); `api/websocket/` (real-time logs/shell/tasks handlers, messaging protocol, connection mgmt); `api/auth_core.rs` (core auth logic); `api/middleware/` (Casbin RBAC authorization); `api/rate_limiting/` (per-tier rate limiting); `docker/state_machine_handlers/` (app lifecycle steps: create dir, save files, docker login, compose up, load balancer config, post actions, wait for containers); `docker/services/` (long-running log streaming and shell sessions); `docker/loadbalancer/` (Traefik/HAProxy config generation); `onepassword/` (resolves `op://` URIs in app env vars via a two-pass lookup + substitution); `oauth/` (OAuth 2.0 device flow for CLI and web flow); `services/authorization/` (Casbin RBAC); `tasks/` (task execution and output streaming); `notification/` (Log, Webhook, Mattermost, GitLab notifications); `static_files.rs` (embedded frontend serving); `metrics/` (collectors for app list, HTTP requests, memory usage, Tokio runtime, etc).

AppState is shared via `Arc` and holds settings, the Docker client (Bollard), the task manager, the authorization service, and metrics collectors.
