---
# scotty-jaq5
title: Fix wildcard dependency for tracing-subscriber
status: completed
type: task
priority: critical
created_at: 2025-12-21T12:44:44Z
updated_at: 2025-12-21T12:44:44Z
---

# Description  tracing-subscriber uses wildcard version "*" which prevents reproducible builds.  # Design  Location: Cargo.toml:36  Current: `tracing-subscriber = "*"` Replace with: `tracing-subscriber = "0.3"`  Impact: Reproducible builds Effort: 5 minutes
