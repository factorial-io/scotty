---
title: Add observability stack to docker-compose
status: closed
priority: 1
issue_type: task
depends_on:
  scotty-6feea: parent-child
created_at: 2025-10-24T23:28:16.207816+00:00
updated_at: 2025-11-24T20:17:25.575601+00:00
closed_at: 2025-10-24T23:38:25.612801+00:00
---

# Description

Create docker-compose.observability.yml with OTel Collector, VictoriaMetrics, and Grafana services. Create OTel Collector configuration file.

# Design

Create docker-compose.observability.yml with:
- otel-collector service (ports 4317/4318)
- victoriametrics service (port 8428)
- grafana service (port 3000)
- Create otel-collector-config.yaml with trace/metrics pipelines
- Add Traefik labels for ddev.site domains

# Acceptance Criteria

- docker-compose up works with observability stack
- OTel Collector receives metrics on port 4317
- VictoriaMetrics stores metrics
- Grafana accessible at grafana.ddev.site
- No port conflicts with existing services
