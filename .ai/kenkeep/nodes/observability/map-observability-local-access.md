---
type: map
title: 'Local observability stack: prerequisite and access URLs'
description: >-
  The observability stack (observability/docker-compose) needs Traefik running
  first for .ddev.site routing; Grafana/Jaeger/VictoriaMetrics are reached via
  *.ddev.site URLs.
tags:
  - observability
  - ddev
  - traefik
  - local-dev
kk_schema_version: 3
kk_id: map-observability-local-access
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Locally, the observability stack depends on Traefik for `.ddev.site` domain routing, so Traefik (`cd apps/traefik && docker compose up -d`) must be started before `cd observability && docker compose up -d`. Once running, services are reachable at: Grafana `http://grafana.ddev.site` (admin/admin, change on first login), Jaeger UI `http://jaeger.ddev.site` (no auth), and VictoriaMetrics `http://vm.ddev.site` (no auth). The Grafana dashboard is provisioned from `observability/grafana/dashboards/scotty-metrics.json` and datasources/dashboards auto-provision from `observability/grafana/provisioning/`.
