---
# scotty-yfhr
title: Add metrics for clone operations in hot paths
status: todo
type: task
priority: critical
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T13:33:37Z
parent: scotty-lbxn
---

# Description  Add tracing/metrics to performance-critical clone operations to identify actual hotspots with real usage data.  # Design  Add instrumentation to measure clone operations in: - AppData access patterns - Settings propagation - State machine handler contexts  Use tracing spans with timing information to identify which clones actually impact performance in production.  This data will help prioritize which clone operations to optimize first.  Impact: Data-driven optimization decisions Effort: 2-3 hours
