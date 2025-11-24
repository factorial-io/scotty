---
title: Add metrics for clone operations in hot paths
status: open
priority: 1
issue_type: task
labels:
- observability
- performance
created_at: 2025-10-26T20:21:10.829701+00:00
updated_at: 2025-11-24T20:17:25.544897+00:00
---

# Description

Add tracing/metrics to performance-critical clone operations to identify actual hotspots with real usage data.

# Design

Add instrumentation to measure clone operations in:
- AppData access patterns
- Settings propagation
- State machine handler contexts

Use tracing spans with timing information to identify which clones actually impact performance in production.

This data will help prioritize which clone operations to optimize first.

Impact: Data-driven optimization decisions
Effort: 2-3 hours
