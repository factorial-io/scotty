//! OpenTelemetry metrics for Scotty
//!
//! This module provides metrics instrumentation for the unified output system.

mod app_list;
mod http;
mod init;
mod instruments;
mod memory;
pub mod tasks;
mod tokio_runtime;
pub mod websocket;

use std::sync::OnceLock;

pub use app_list::sample_app_list_metrics;
pub use http::http_metrics_middleware;
pub use init::init_metrics;
pub use instruments::ScottyMetrics;
pub use memory::sample_memory_metrics;
pub use tokio_runtime::{sample_tokio_metrics, spawn_instrumented};

/// Global metrics instance
static METRICS: OnceLock<ScottyMetrics> = OnceLock::new();

/// Get the global metrics instance if initialized
pub fn get_metrics() -> Option<&'static ScottyMetrics> {
    METRICS.get()
}

/// Set the global metrics instance (called during initialization)
pub(crate) fn set_metrics(metrics: ScottyMetrics) {
    let _ = METRICS.set(metrics);
}
