---
title: Add OpenTelemetry metrics dependencies to workspace
status: closed
priority: 1
issue_type: task
depends_on:
  scotty-6feea: parent-child
created_at: 2025-10-24T23:28:15.598548+00:00
updated_at: 2025-11-24T20:17:25.573818+00:00
closed_at: 2025-10-24T23:32:41.399679+00:00
---

# Description

Add metrics feature to opentelemetry and opentelemetry-otlp crates in workspace Cargo.toml. Enable metrics support in opentelemetry_sdk.

# Design

Update workspace Cargo.toml:
- opentelemetry: Add "metrics" feature
- opentelemetry_sdk: Add "metrics" to features array
- opentelemetry-otlp: Add "metrics" feature

# Acceptance Criteria

- Cargo.toml updated with metrics features
- cargo check passes
- No breaking changes to existing trace functionality
