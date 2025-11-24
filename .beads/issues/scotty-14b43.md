---
title: Add TaskManager metrics for task tracking
status: closed
priority: 2
issue_type: task
depends_on:
  scotty-06fec: blocks
created_at: 2025-10-25T00:37:07.454119+00:00
updated_at: 2025-11-24T20:17:25.572273+00:00
closed_at: 2025-10-25T13:29:19.632177+00:00
---

# Description

Instrument TaskManager to track number of tasks, their states (running/finished/failed), durations, and success rates.

# Design

Add TaskManager metrics to ScottyMetrics struct:
- tasks_total (Counter) - Total tasks created
- tasks_by_state (Gauge) - Current tasks grouped by state labels:
  * state="running"
  * state="finished"
  * state="failed"
- task_duration_seconds (Histogram) - Task execution time
- task_failures_total (Counter) - Failed tasks counter

Implementation approach:
1. Add metrics to scotty/src/metrics/instruments.rs
2. Instrument scotty/src/tasks/manager.rs:
   - add_task(): increment tasks_total, update state gauge
   - set_task_finished(): update state gauges, record duration histogram
   - Optional: background sampler every 30s to sync gauges with HashMap state

Data sources:
- TaskManager.processes HashMap size = total active tasks
- TaskDetails.state: Running | Finished | Failed
- TaskDetails.start_time + finish_time for duration calculation
- TaskDetails.last_exit_code for success/failure tracking

# Acceptance Criteria

- Task metrics exported to OTLP
- Metrics accurately reflect task states
- Duration tracking works correctly
- Failed vs successful tasks distinguishable
- Dashboard panels created for task monitoring
- No performance degradation
