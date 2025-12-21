---
# scotty-1flf
title: Create metrics module with ScottyMetrics struct
status: completed
type: task
priority: critical
created_at: 2025-12-21T12:44:44Z
updated_at: 2025-12-21T12:44:48Z
parent: scotty-k1mu
blocking:
    - scotty-tfk7
    - scotty-agx2
    - scotty-6gg2
    - scotty-7voi
    - scotty-8tbf
    - scotty-w8eg
    - scotty-aauq
    - scotty-pvdo
    - scotty-5bjr
---

# Description  Create scotty/src/metrics/mod.rs with ScottyMetrics struct containing all metric instruments (counters, gauges, histograms) for unified output system monitoring.  # Design  Create metrics module with: - ScottyMetrics struct with all instruments - init_metrics() function to set up OTLP exporter - Metrics for: log streams, shell sessions, WebSocket, tasks, system health - Uses opentelemetry::metrics API (Counter, Gauge, Histogram)  # Acceptance Criteria  - metrics/mod.rs created and compiles - ScottyMetrics struct has all planned metrics - init_metrics() successfully initializes MeterProvider - Metrics can be recorded without panics
