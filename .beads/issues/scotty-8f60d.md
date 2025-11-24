---
title: Fix wildcard dependency for tracing-subscriber
status: closed
priority: 1
issue_type: chore
assignee: claude
labels:
- dependencies
created_at: 2025-10-26T20:21:10.538480+00:00
updated_at: 2025-11-24T20:17:25.576761+00:00
closed_at: 2025-10-26T21:00:48.493544+00:00
---

# Description

tracing-subscriber uses wildcard version "*" which prevents reproducible builds.

# Design

Location: Cargo.toml:36

Current: `tracing-subscriber = "*"`
Replace with: `tracing-subscriber = "0.3"`

Impact: Reproducible builds
Effort: 5 minutes
