---
title: Instrument shell service with metrics
status: closed
priority: 1
issue_type: task
depends_on:
  scotty-06fec: blocks
  scotty-6feea: parent-child
created_at: 2025-10-24T23:28:15.964474+00:00
updated_at: 2025-11-24T20:17:25.551854+00:00
closed_at: 2025-10-26T17:37:15.004569+00:00
---

# Description

Add metrics recording to ShellService for session counts, durations, errors, and timeouts.

# Design

Instrument ShellService:
- Track shell_sessions_active gauge
- Increment shell_sessions_total counter
- Record shell_session_duration histogram
- Track shell_session_errors and timeouts

# Acceptance Criteria

- Session metrics recorded correctly
- Duration tracking works
- Timeout cases tracked separately
- Memory leak tests pass
