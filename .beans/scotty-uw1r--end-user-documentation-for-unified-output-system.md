---
# scotty-uw1r
title: End-user documentation for unified output system
status: in-progress
type: task
priority: normal
created_at: 2025-12-21T12:44:44Z
updated_at: 2025-12-21T14:54:22Z
parent: scotty-rsgr
---

# Description

Write comprehensive end-user documentation for the logs and shell features. Technical PRD exists but user-facing docs are missing.

# Analysis

Current state as of 2025-12-21:
- **app:logs CLI**: ✅ Fully documented in docs/content/cli.md (lines 83-119)
- **app:shell CLI**: ✅ Fully documented in docs/content/cli.md (lines 121-162)
- **Web UI log viewer**: ✅ Documented with screenshots
- **Web UI shell access**: ❌ NOT IMPLEMENTED - blocked until frontend shell UI is built
- **Screenshots**: ✅ Added (dashboard.png, app-detail-page.png, log-viewer.png)
- **Security best practices**: ✅ Added to docs/content/cli.md
- **Troubleshooting guide**: ✅ Added to docs/content/cli.md

# Checklist

## Already Complete (CLI docs)
- [x] app:logs documented with all options (--follow, --lines, --since, --until, --timestamps)
- [x] app:shell documented with all options (--command, --shell)

## Web UI Documentation
- [x] Document how to access service logs from the dashboard
- [x] Add section explaining the log viewer UI features
- [ ] Note: Shell UI documentation blocked until frontend implementation complete

## Security Best Practices
- [x] Document shell permission requirements
- [x] Document logs permission requirements
- [x] Add audit/security considerations for shell access
- [x] Note container isolation implications

## Troubleshooting Guide
- [x] WebSocket connection failures
- [x] "Permission denied" errors for logs/shell
- [x] Log stream not starting
- [x] Shell session timeouts

## Screenshots
- [x] Capture log viewer UI screenshot (log-viewer.png)
- [x] Capture service detail page screenshot (app-detail-page.png)
- [x] Capture dashboard screenshot (dashboard.png)

# Blocked Items

The following items are blocked until parent epic scotty-rsgr completes shell UI:
- Web UI documentation for shell access
- Shell UI screenshots

# Acceptance Criteria

- [x] app:logs documented with all options
- [x] app:shell documented (CLI complete)
- [x] Screenshots of web UI features
- [x] Security guidelines clear
- [x] Common issues documented
- [ ] Published to docs site