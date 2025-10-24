//! OpenTelemetry metrics for Scotty
//!
//! This module provides metrics instrumentation for the unified output system.

mod init;
mod instruments;

use std::sync::OnceLock;

pub use init::init_metrics;
pub use instruments::ScottyMetrics;

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
