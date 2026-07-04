---
type: map
title: Observability stack data retention limits
description: >-
  VictoriaMetrics retains metrics 30 days by default (configurable); Jaeger
  traces are in-memory only and lost on restart.
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
VictoriaMetrics retains metrics for 30 days by default; this is configurable via the `-retentionPeriod` flag on the `victoriametrics` service in `observability/docker-compose.yml`.

Jaeger stores traces in-memory only in this local setup, so trace data is lost whenever the Jaeger container restarts.
