---
# scotty-xaor
title: Add observability stack to docker-compose
status: completed
type: task
priority: critical
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T12:44:48Z
parent: scotty-k1mu
---

# Description  Create docker-compose.observability.yml with OTel Collector, VictoriaMetrics, and Grafana services. Create OTel Collector configuration file.  # Design  Create docker-compose.observability.yml with: - otel-collector service (ports 4317/4318) - victoriametrics service (port 8428) - grafana service (port 3000) - Create otel-collector-config.yaml with trace/metrics pipelines - Add Traefik labels for ddev.site domains  # Acceptance Criteria  - docker-compose up works with observability stack - OTel Collector receives metrics on port 4317 - VictoriaMetrics stores metrics - Grafana accessible at grafana.ddev.site - No port conflicts with existing services
