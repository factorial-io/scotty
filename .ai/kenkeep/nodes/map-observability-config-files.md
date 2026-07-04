---
type: map
title: Observability stack config file locations
description: >-
  docker-compose.yml, otel-collector-config.yaml, and grafana/
  provisioning/dashboards dirs define the observability stack's setup.
tags:
  - observability
  - grafana
  - configuration
kk_schema_version: 3
kk_id: map-observability-config-files
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
In `observability/`, `docker-compose.yml` defines the service definitions and resource limits for the stack, and `otel-collector-config.yaml` defines the OpenTelemetry Collector's pipeline configuration (routing OTLP input to Jaeger and VictoriaMetrics).

Grafana is auto-provisioned: `grafana/provisioning/datasources/` configures the VictoriaMetrics datasource, and `grafana/dashboards/scotty-metrics.json` is the pre-built Scotty metrics dashboard, reachable in the UI via Dashboards → Scotty Metrics.
