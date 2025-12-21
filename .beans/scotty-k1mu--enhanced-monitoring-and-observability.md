---
# scotty-k1mu
title: Enhanced monitoring and observability
status: todo
type: feature
priority: normal
created_at: 2025-12-21T12:44:44Z
updated_at: 2025-12-21T12:44:47Z
parent: scotty-rsgr
---

# Description  Add comprehensive monitoring for the unified output system. Basic logging exists but metrics and tracing are incomplete.  # Design  Implementation using OpenTelemetry Collector + VictoriaMetrics architecture.  Architecture: - Scotty exports OTLP metrics to OTel Collector (port 4317) - OTel Collector routes traces to Jaeger, metrics to VictoriaMetrics - Grafana visualizes metrics from VictoriaMetrics - Total memory overhead: ~180-250 MB  See docs/research/otel-metrics-backend-evaluation.md for complete research and rationale.  # Acceptance Criteria  - Prometheus metrics exported - Grafana dashboard created - Tracing spans for log/shell operations - Memory usage tracked - Error rates monitored  # Notes  Chosen solution: OTel Collector + VictoriaMetrics for lightweight, OpenTelemetry-native metrics collection. Integrates with existing Jaeger setup.
