---
# scotty-agx2
title: Add memory usage metrics
status: completed
type: task
priority: critical
created_at: 2025-12-21T12:44:44Z
updated_at: 2025-12-21T12:44:44Z
---

# Description  Track scotty's memory usage (heap allocated, RSS, etc.) to monitor resource consumption and detect memory leaks.  # Design  Add memory metrics to ScottyMetrics struct: - memory_heap_bytes (Gauge) - heap allocated memory - memory_rss_bytes (Gauge) - resident set size - Consider using jemalloc or system metrics crate  Options: 1. Use `sysinfo` crate for cross-platform process metrics 2. Use jemalloc stats if using jemalloc allocator 3. Sample memory every 30s-60s to avoid overhead  Location:  - Add metrics to scotty/src/metrics/instruments.rs - Add sampling task to scotty/src/main.rs or metrics/mod.rs - Record metrics periodically in background task  # Acceptance Criteria  - Memory metrics exported to OTLP - Metrics update at reasonable interval (30-60s) - Minimal performance overhead - Works on all platforms (Linux, macOS) - Dashboard panel created for memory tracking
