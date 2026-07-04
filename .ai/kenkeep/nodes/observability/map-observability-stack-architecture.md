---
type: map
title: Observability stack architecture
description: >-
  Scotty exports OTLP telemetry to an OpenTelemetry Collector, which routes
  traces to Jaeger and metrics to VictoriaMetrics, visualized in Grafana.
tags:
  - observability
  - opentelemetry
  - metrics
  - tracing
  - grafana
kk_schema_version: 3
kk_id: map-observability-stack-architecture
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Scotty's observability stack has four components: the OpenTelemetry Collector receives telemetry from Scotty over OTLP (gRPC, port 4317) and routes it to backends; VictoriaMetrics is the time-series database storing metrics (30-day retention by default); Jaeger is the distributed tracing backend for request traces/spans; Grafana is the visualization layer with pre-configured dashboards reading from VictoriaMetrics.

The stack is lightweight by design: roughly 180-250 MB memory and under 5% CPU on modern systems, with 1-2 GB disk for 30 days of metrics.
