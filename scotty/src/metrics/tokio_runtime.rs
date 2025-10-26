use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_metrics::TaskMonitor;

/// Global task monitor for tracking Tokio task metrics
static TASK_MONITOR: once_cell::sync::Lazy<Arc<RwLock<TaskMonitor>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(TaskMonitor::new())));

/// Spawn a task instrumented with the global TaskMonitor
///
/// This is a convenience wrapper around `tokio::spawn` that automatically
/// instruments the task with our metrics TaskMonitor.
pub async fn spawn_instrumented<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let monitor = TASK_MONITOR.read().await;
    tokio::spawn(monitor.instrument(future))
}

/// Sample Tokio task metrics and record them
///
/// Called periodically by the scheduler to track instrumented tasks and worker threads.
/// Note: Only tasks that are instrumented with the TaskMonitor will be tracked.
pub async fn sample_tokio_metrics() {
    let monitor = TASK_MONITOR.read().await;
    let mut intervals = monitor.intervals();

    if let Some(metrics) = intervals.next() {
        tracing::debug!(
            "Tokio metrics sample - active_tasks={}, dropped={}, polls={}, slow_polls={}, scheduled={}, poll_duration={:?}, idle_duration={:?}",
            metrics.instrumented_count,
            metrics.dropped_count,
            metrics.total_poll_count,
            metrics.total_slow_poll_count,
            metrics.total_scheduled_count,
            metrics.total_poll_duration,
            metrics.total_idle_duration
        );

        // Record metrics if available
        if let Some(m) = super::get_metrics() {
            // Task counts
            m.tokio_active_tasks_count
                .record(metrics.instrumented_count, &[]);
            m.tokio_tasks_dropped.add(metrics.dropped_count, &[]);
            m.tokio_workers_count.record(num_cpus::get() as u64, &[]);

            // Poll metrics
            m.tokio_poll_count.add(metrics.total_poll_count, &[]);
            m.tokio_slow_poll_count
                .add(metrics.total_slow_poll_count, &[]);

            // Duration metrics (convert from Duration to seconds)
            // For histograms, we record the total duration across all events in this interval
            // The histogram will track the distribution of these aggregate values
            let poll_duration_secs = metrics.total_poll_duration.as_secs_f64();
            if poll_duration_secs > 0.0 {
                m.tokio_poll_duration.record(poll_duration_secs, &[]);
            }

            let idle_duration_secs = metrics.total_idle_duration.as_secs_f64();
            if idle_duration_secs > 0.0 {
                m.tokio_idle_duration.record(idle_duration_secs, &[]);
            }

            // Scheduling metrics
            m.tokio_scheduled_count
                .add(metrics.total_scheduled_count, &[]);

            // First poll delay
            let first_poll_delay_secs = metrics.total_first_poll_delay.as_secs_f64();
            if first_poll_delay_secs > 0.0 {
                m.tokio_first_poll_delay
                    .record(first_poll_delay_secs, &[]);
            }
        }
    }
}
