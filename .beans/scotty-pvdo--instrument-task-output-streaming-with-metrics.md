---
# scotty-pvdo
title: Instrument task output streaming with metrics
status: completed
type: task
priority: normal
created_at: 2025-12-21T12:44:46Z
updated_at: 2025-12-21T12:44:46Z
---

# Description  Add metrics recording to TaskOutputStreamingService for task counts, durations, errors, and output volume.  # Design  Instrument TaskOutputStreamingService similar to LogStreamingService: - Track tasks_active gauge (current running tasks) - Track tasks_total counter (cumulative task count) - Add task_output_lines counter (throughput tracking) - Add task_errors counter (error tracking) - Consider task_duration histogram if tasks have clear lifecycle  Location: scotty/src/tasks/output_streaming.rs Pattern: Use metrics::get_metrics() to access global metrics instance  # Acceptance Criteria  - Metrics recorded at task start/end - Output lines counted - Error cases tracked - No performance degradation - Code compiles and tests pass
