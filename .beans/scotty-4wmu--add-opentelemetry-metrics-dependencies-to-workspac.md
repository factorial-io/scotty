---
# scotty-4wmu
title: Add OpenTelemetry metrics dependencies to workspace
status: completed
type: task
priority: critical
created_at: 2025-12-21T12:44:47Z
updated_at: 2025-12-21T12:44:48Z
parent: scotty-k1mu
---

# Description  Add metrics feature to opentelemetry and opentelemetry-otlp crates in workspace Cargo.toml. Enable metrics support in opentelemetry_sdk.  # Design  Update workspace Cargo.toml: - opentelemetry: Add "metrics" feature - opentelemetry_sdk: Add "metrics" to features array - opentelemetry-otlp: Add "metrics" feature  # Acceptance Criteria  - Cargo.toml updated with metrics features - cargo check passes - No breaking changes to existing trace functionality
