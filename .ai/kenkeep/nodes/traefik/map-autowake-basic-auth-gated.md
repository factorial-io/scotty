---
type: map
title: Autowake starts stopped apps from the catch-all, gated by basic auth
description: >-
  Apps with autowake: true are started by Scotty's catch-all handler when the
  request carries the app's basic-auth credentials; no Scotty login, no output.
tags:
  - landing-page
  - traefik
  - security
  - autowake
kk_schema_version: 3
kk_id: map-autowake-basic-auth-gated
kk_derived_from: []
kk_relates_to:
  - map-default-backend-landing-page
  - map-default-backend-security-model
kk_depends_on: []
kk_confidence: high
---
`AppSettings.autowake` (scottyctl `app:create --autowake`) makes `landing.rs` start a stopped app instead of redirecting to the landing page. The app's own `basic_auth` is the only gate: Scotty compares the `Authorization: Basic` header in constant time, answers `401` with `WWW-Authenticate: Basic realm="traefik"` (Traefik's default realm, so the browser reuses credentials once the app's route is live), and on a match calls `wake_app` (same steps as `run_app`, no `AppStarted` notification) and returns a self-refreshing `503`. Creation rejects autowake without basic auth; at runtime that state falls back to the landing redirect. App status is derived from containers and stays `Stopped` until they exist, so `AppState::begin_wake` holds a per-app `WakeGuard` in an in-memory set until the start steps finish (dropped on failure or panic too).
