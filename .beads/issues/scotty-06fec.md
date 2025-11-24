---
title: Create metrics module with ScottyMetrics struct
status: closed
priority: 1
issue_type: task
depends_on:
  scotty-6feea: parent-child
  scotty-e2060: blocks
created_at: 2025-10-24T23:28:15.721881+00:00
updated_at: 2025-11-24T20:17:25.578481+00:00
closed_at: 2025-10-24T23:50:03.687196+00:00
---

# Description

Create scotty/src/metrics/mod.rs with ScottyMetrics struct containing all metric instruments (counters, gauges, histograms) for unified output system monitoring.

# Design

Create metrics module with:
- ScottyMetrics struct with all instruments
- init_metrics() function to set up OTLP exporter
- Metrics for: log streams, shell sessions, WebSocket, tasks, system health
- Uses opentelemetry::metrics API (Counter, Gauge, Histogram)

# Acceptance Criteria

- metrics/mod.rs created and compiles
- ScottyMetrics struct has all planned metrics
- init_metrics() successfully initializes MeterProvider
- Metrics can be recorded without panics
