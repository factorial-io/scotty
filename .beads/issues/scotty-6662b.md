---
title: Instrument log streaming service with metrics
status: closed
priority: 1
issue_type: task
depends_on:
  scotty-06fec: blocks
  scotty-6feea: parent-child
created_at: 2025-10-24T23:28:15.847014+00:00
updated_at: 2025-11-24T20:17:25.583118+00:00
closed_at: 2025-10-24T23:57:04.063722+00:00
---

# Description

Add metrics recording to LogStreamingService for stream counts, durations, errors, and bytes transferred.

# Design

Instrument LogStreamingService:
- Increment log_streams_active on stream start
- Increment log_streams_total counter
- Record log_stream_duration on completion
- Track log_lines_received and log_stream_bytes
- Increment log_stream_errors on failures

# Acceptance Criteria

- Metrics recorded at stream start/end
- Duration measured accurately
- Error cases increment error counter
- No performance degradation
