---
title: Create Grafana dashboards for Scotty metrics
status: closed
priority: 2
issue_type: task
depends_on:
  scotty-6feea: parent-child
  scotty-23c1b: blocks
created_at: 2025-10-24T23:28:16.355104+00:00
updated_at: 2025-11-24T20:17:25.586767+00:00
closed_at: 2025-10-25T13:21:28.683425+00:00
---

# Description

Create Grafana dashboard JSON and provisioning config showing unified output system metrics with panels for log streams, shell sessions, WebSocket connections, and tasks.

# Design

Create Grafana assets:
- grafana/provisioning/datasources/datasources.yaml (VictoriaMetrics + Jaeger)
- grafana/provisioning/dashboards/dashboards.yaml
- grafana/dashboards/scotty-unified-output.json
- Panels: active streams, session durations, message rates, error rates

# Acceptance Criteria

- Grafana dashboard loads automatically
- All panels show live data
- VictoriaMetrics datasource works
- Dashboard is intuitive and useful
- Export JSON to git
