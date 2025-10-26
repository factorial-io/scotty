use std::sync::atomic::{AtomicI64, Ordering};

/// Task metrics instrumentation helpers
///
/// These functions provide a clean API for recording task metrics
/// without cluttering the business logic with metrics implementation details.

/// Track active task output streams in memory
static ACTIVE_TASK_STREAMS: AtomicI64 = AtomicI64::new(0);

/// Record a task output stream started
pub fn record_stream_started() {
    let count = ACTIVE_TASK_STREAMS.fetch_add(1, Ordering::Relaxed) + 1;
    if let Some(m) = super::get_metrics() {
        m.tasks_active.record(count, &[]);
    }
}

/// Record a task output stream ended
pub fn record_stream_ended() {
    let count = ACTIVE_TASK_STREAMS.fetch_sub(1, Ordering::Relaxed) - 1;
    if let Some(m) = super::get_metrics() {
        m.tasks_active.record(count, &[]);
    }
}

/// Record task output lines sent
pub fn record_output_lines(count: u64) {
    if count > 0 {
        if let Some(m) = super::get_metrics() {
            m.task_output_lines.add(count, &[]);
        }
    }
}

/// Record a task failure/error
pub fn record_task_failure() {
    if let Some(m) = super::get_metrics() {
        m.task_failures.add(1, &[]);
    }
}
