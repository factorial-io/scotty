---
# scotty-7voi
title: Add TaskManager metrics for task tracking
status: completed
type: task
priority: high
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T12:44:45Z
---

# Description  Instrument TaskManager to track number of tasks, their states (running/finished/failed), durations, and success rates.  # Design  Add TaskManager metrics to ScottyMetrics struct: - tasks_total (Counter) - Total tasks created - tasks_by_state (Gauge) - Current tasks grouped by state labels:   * state="running"   * state="finished"   * state="failed" - task_duration_seconds (Histogram) - Task execution time - task_failures_total (Counter) - Failed tasks counter  Implementation approach: 1. Add metrics to scotty/src/metrics/instruments.rs 2. Instrument scotty/src/tasks/manager.rs:    - add_task(): increment tasks_total, update state gauge    - set_task_finished(): update state gauges, record duration histogram    - Optional: background sampler every 30s to sync gauges with HashMap state  Data sources: - TaskManager.processes HashMap size = total active tasks - TaskDetails.state: Running | Finished | Failed - TaskDetails.start_time + finish_time for duration calculation - TaskDetails.last_exit_code for success/failure tracking  # Acceptance Criteria  - Task metrics exported to OTLP - Metrics accurately reflect task states - Duration tracking works correctly - Failed vs successful tasks distinguishable - Dashboard panels created for task monitoring - No performance degradation
