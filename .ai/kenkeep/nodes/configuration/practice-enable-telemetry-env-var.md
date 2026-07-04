---
type: practice
title: Enable metrics/traces export via SCOTTY__TELEMETRY
description: >-
  Set SCOTTY__TELEMETRY=metrics,traces (or just metrics/traces) to have Scotty
  export OTLP telemetry.
tags:
  - observability
  - config
  - env-vars
kk_schema_version: 3
kk_id: practice-enable-telemetry-env-var
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
Telemetry export is opt-in and controlled by the `SCOTTY__TELEMETRY` environment variable. Set it to `metrics`, `traces`, or `metrics,traces` to enable the corresponding OTLP exports from the running Scotty server (e.g. `SCOTTY__TELEMETRY=metrics,traces cargo run --bin scotty`, or as an env var in production docker-compose/`.env`).
