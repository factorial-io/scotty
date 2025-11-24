---
title: Add Tokio runtime metrics
status: closed
priority: 3
issue_type: task
depends_on:
  scotty-06fec: blocks
created_at: 2025-10-25T00:30:22.111330+00:00
updated_at: 2025-11-24T20:17:25.565065+00:00
closed_at: 2025-10-25T14:17:30.994921+00:00
---

# Description

Track Tokio async runtime statistics like active tasks, worker threads, idle time, and task queue depth to monitor async performance.

# Design

Add Tokio runtime metrics to ScottyMetrics:
- tokio_tasks_active (Gauge) - currently spawned tasks
- tokio_workers_count (Gauge) - number of worker threads
- tokio_workers_idle (Gauge) - idle workers
- tokio_tasks_spawned_total (Counter) - total tasks spawned

Implementation options:
1. Use tokio-metrics crate (official tokio metrics)
2. Use tokio::runtime::Handle::metrics() (requires unstable features)
3. Use tokio-console integration via console-subscriber

Recommended: tokio-metrics crate
- Add tokio-metrics to Cargo.toml
- Create background task to sample runtime metrics
- Record every 10-30s

Location:
- Add metrics to scotty/src/metrics/instruments.rs
- Add tokio metrics sampler in scotty/src/metrics/tokio.rs
- Spawn sampling task in main.rs after runtime creation

# Acceptance Criteria

- Tokio runtime metrics exported to OTLP
- Metrics show active tasks and worker state
- Minimal performance overhead
- Code compiles with stable Rust
- Dashboard panel created for runtime monitoring
