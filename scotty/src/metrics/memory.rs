use sysinfo::{ProcessesToUpdate, System};
use tracing::{debug, warn};

/// Sample current memory usage and record metrics
///
/// Called periodically by the scheduler to minimize overhead while providing
/// useful monitoring data for detecting memory leaks and resource usage trends.
pub async fn sample_memory_metrics() {
    let mut system = System::new_all();
    let pid = match sysinfo::get_current_pid() {
        Ok(pid) => pid,
        Err(e) => {
            warn!("Failed to get current PID: {}", e);
            return;
        }
    };

    // Refresh process information for the current PID
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), false);

    if let Some(process) = system.process(pid) {
        let rss_bytes = process.memory();
        let virtual_bytes = process.virtual_memory();

        debug!(
            "Memory usage: RSS={} MB, Virtual={} MB",
            rss_bytes / 1024 / 1024,
            virtual_bytes / 1024 / 1024
        );

        super::metrics().record_memory_rss_bytes(rss_bytes);
        super::metrics().record_memory_virtual_bytes(virtual_bytes);
    } else {
        warn!("Failed to get process information for memory metrics");
    }
}
