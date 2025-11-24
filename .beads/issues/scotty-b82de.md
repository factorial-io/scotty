---
title: Instrument task output streaming with metrics
status: closed
priority: 3
issue_type: task
depends_on:
  scotty-06fec: blocks
created_at: 2025-10-25T00:29:59.988921+00:00
updated_at: 2025-11-24T20:17:25.580684+00:00
closed_at: 2025-10-26T17:10:42.533960+00:00
---

# Description

Add metrics recording to TaskOutputStreamingService for task counts, durations, errors, and output volume.

# Design

Instrument TaskOutputStreamingService similar to LogStreamingService:
- Track tasks_active gauge (current running tasks)
- Track tasks_total counter (cumulative task count)
- Add task_output_lines counter (throughput tracking)
- Add task_errors counter (error tracking)
- Consider task_duration histogram if tasks have clear lifecycle

Location: scotty/src/tasks/output_streaming.rs
Pattern: Use metrics::get_metrics() to access global metrics instance

# Acceptance Criteria

- Metrics recorded at task start/end
- Output lines counted
- Error cases tracked
- No performance degradation
- Code compiles and tests pass
