//! OpenTelemetry metrics for Scotty
//!
//! This module provides metrics instrumentation for the unified output system.

mod init;
mod instruments;

pub use init::init_metrics;
pub use instruments::ScottyMetrics;
