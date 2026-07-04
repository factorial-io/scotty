---
type: practice
title: Observability backends are swappable via open standards
description: >-
  Scotty's telemetry uses OTLP, PromQL, and W3C Trace Context so any component
  (VictoriaMetrics, Jaeger, Grafana) can be replaced with a compatible
  alternative.
tags:
  - observability
  - prometheus
  - architecture
kk_schema_version: 3
kk_id: practice-observability-stack-swappable
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
All Scotty metrics are fully Prometheus-compatible: OpenTelemetry names like `scotty.metric.name` map to Prometheus-style `scotty_metric_name_total`, using standard Counter/Gauge/Histogram/UpDownCounter types with attributes becoming labels. Because the stack relies on open standards (OTLP, PromQL, W3C Trace Context), any component is replaceable — e.g. swap VictoriaMetrics for Prometheus/Thanos/Cortex/Datadog by changing the OTel Collector exporter config, or point the collector at multiple backends at once.

VictoriaMetrics is the default specifically for development convenience (lower memory, single binary, Prometheus-compatible, free); production deployments can swap in Prometheus or another backend if preferred.
