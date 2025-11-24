---
title: Enhanced monitoring and observability
status: open
priority: 3
issue_type: task
depends_on:
  scotty-541fa: parent-child
created_at: 2025-10-24T22:58:22.135422+00:00
updated_at: 2025-11-24T20:17:25.563083+00:00
---

# Description

Add comprehensive monitoring for the unified output system. Basic logging exists but metrics and tracing are incomplete.

# Design

Implementation using OpenTelemetry Collector + VictoriaMetrics architecture.

Architecture:
- Scotty exports OTLP metrics to OTel Collector (port 4317)
- OTel Collector routes traces to Jaeger, metrics to VictoriaMetrics
- Grafana visualizes metrics from VictoriaMetrics
- Total memory overhead: ~180-250 MB

See docs/research/otel-metrics-backend-evaluation.md for complete research and rationale.

# Acceptance Criteria

- Prometheus metrics exported
- Grafana dashboard created
- Tracing spans for log/shell operations
- Memory usage tracked
- Error rates monitored

# Notes

Chosen solution: OTel Collector + VictoriaMetrics for lightweight, OpenTelemetry-native metrics collection. Integrates with existing Jaeger setup.
