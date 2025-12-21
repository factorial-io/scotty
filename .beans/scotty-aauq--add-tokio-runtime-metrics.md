---
# scotty-aauq
title: Add Tokio runtime metrics
status: completed
type: task
priority: normal
created_at: 2025-12-21T12:44:46Z
updated_at: 2025-12-21T12:44:46Z
---

# Description  Track Tokio async runtime statistics like active tasks, worker threads, idle time, and task queue depth to monitor async performance.  # Design  Add Tokio runtime metrics to ScottyMetrics: - tokio_tasks_active (Gauge) - currently spawned tasks - tokio_workers_count (Gauge) - number of worker threads - tokio_workers_idle (Gauge) - idle workers - tokio_tasks_spawned_total (Counter) - total tasks spawned  Implementation options: 1. Use tokio-metrics crate (official tokio metrics) 2. Use tokio::runtime::Handle::metrics() (requires unstable features) 3. Use tokio-console integration via console-subscriber  Recommended: tokio-metrics crate - Add tokio-metrics to Cargo.toml - Create background task to sample runtime metrics - Record every 10-30s  Location: - Add metrics to scotty/src/metrics/instruments.rs - Add tokio metrics sampler in scotty/src/metrics/tokio.rs - Spawn sampling task in main.rs after runtime creation  # Acceptance Criteria  - Tokio runtime metrics exported to OTLP - Metrics show active tasks and worker state - Minimal performance overhead - Code compiles with stable Rust - Dashboard panel created for runtime monitoring
