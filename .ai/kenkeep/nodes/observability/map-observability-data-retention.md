---
type: map
title: Observability stack data retention limits
description: >-
  VictoriaMetrics retains metrics 7 days (configurable); Jaeger traces are
  in-memory only and lost on restart.
tags:
  - observability
  - jaeger
  - victoriametrics
kk_schema_version: 3
kk_id: map-observability-data-retention
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
VictoriaMetrics retains metrics for 7 days (`--retentionPeriod=7d` on the `victoriametrics` service in `observability/docker-compose.yml`); change that flag to adjust it.

Jaeger stores traces in-memory only in this local setup, so trace data is lost whenever the Jaeger container restarts.
