use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_metrics::TaskMonitor;
use tracing::debug;

/// Global task monitor for tracking Tokio task metrics
static TASK_MONITOR: once_cell::sync::Lazy<Arc<RwLock<TaskMonitor>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(TaskMonitor::new())));

/// Get the global task monitor
///
/// This should be used to instrument tasks throughout the application.
#[allow(dead_code)] // Reserved for future task instrumentation
pub fn get_task_monitor() -> Arc<RwLock<TaskMonitor>> {
    TASK_MONITOR.clone()
}

/// Sample Tokio task metrics and record them
///
/// Called periodically by the scheduler to track instrumented tasks and worker threads.
/// Note: Only tasks that are instrumented with the TaskMonitor will be tracked.
pub async fn sample_tokio_metrics() {
    let monitor = TASK_MONITOR.read().await;
    let mut intervals = monitor.intervals();

    if let Some(metrics) = intervals.next() {
        let instrumented_count = metrics.instrumented_count;

        debug!(
            "Tokio task metrics - instrumented tasks: {}, workers: {}",
            instrumented_count,
            num_cpus::get()
        );

        // Record metrics if available
        if let Some(m) = super::get_metrics() {
            m.tokio_active_tasks_count.record(instrumented_count, &[]);
            // Note: Worker count is set to a constant since we can't get it from stable APIs
            m.tokio_workers_count.record(num_cpus::get() as u64, &[]);
        }
    }
}
