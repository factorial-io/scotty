---
# scotty-h3k3
title: Create Grafana dashboards for Scotty metrics
status: completed
type: task
priority: high
created_at: 2025-12-21T12:44:44Z
updated_at: 2025-12-21T12:44:47Z
parent: scotty-k1mu
---

# Description  Create Grafana dashboard JSON and provisioning config showing unified output system metrics with panels for log streams, shell sessions, WebSocket connections, and tasks.  # Design  Create Grafana assets: - grafana/provisioning/datasources/datasources.yaml (VictoriaMetrics + Jaeger) - grafana/provisioning/dashboards/dashboards.yaml - grafana/dashboards/scotty-unified-output.json - Panels: active streams, session durations, message rates, error rates  # Acceptance Criteria  - Grafana dashboard loads automatically - All panels show live data - VictoriaMetrics datasource works - Dashboard is intuitive and useful - Export JSON to git
