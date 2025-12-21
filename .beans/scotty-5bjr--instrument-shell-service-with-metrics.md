---
# scotty-5bjr
title: Instrument shell service with metrics
status: completed
type: task
priority: critical
created_at: 2025-12-21T12:44:46Z
updated_at: 2025-12-21T12:44:46Z
---

# Description  Add metrics recording to ShellService for session counts, durations, errors, and timeouts.  # Design  Instrument ShellService: - Track shell_sessions_active gauge - Increment shell_sessions_total counter - Record shell_session_duration histogram - Track shell_session_errors and timeouts  # Acceptance Criteria  - Session metrics recorded correctly - Duration tracking works - Timeout cases tracked separately - Memory leak tests pass
