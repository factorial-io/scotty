//! OpenTelemetry metrics for Scotty
//!
//! This module provides metrics instrumentation for the unified output system.

// Recorder trait (always available)
mod recorder_trait;
use recorder_trait::MetricsRecorder;

// No-op recorder (always available for fallback)
mod noop;

#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
mod app_list;
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
mod http;
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
mod init;
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
mod instruments;
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
mod memory;
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
mod otel_recorder;
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
mod tokio_runtime;

use std::sync::OnceLock;

// Telemetry exports
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
pub use app_list::sample_app_list_metrics;
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
pub use http::http_metrics_middleware;
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
pub use init::init_metrics;
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
pub use memory::sample_memory_metrics;
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
pub use tokio_runtime::{sample_tokio_metrics, spawn_instrumented};

// Global recorder
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
static RECORDER: OnceLock<otel_recorder::OtelRecorder> = OnceLock::new();

#[cfg(not(any(feature = "telemetry-grpc", feature = "telemetry-http")))]
static RECORDER: OnceLock<noop::NoOpRecorder> = OnceLock::new();

// Fallback no-op recorder for when telemetry isn't initialized (e.g., tests)
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
static NOOP_FALLBACK: noop::NoOpRecorder = noop::NoOpRecorder::new();

/// Get the global metrics recorder
///
/// Returns the initialized recorder if available, otherwise returns a no-op fallback.
/// This ensures metrics calls never panic, even in tests where init_metrics() isn't called.
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
pub(crate) fn metrics() -> &'static dyn MetricsRecorder {
    RECORDER
        .get()
        .map(|r| r as &'static dyn MetricsRecorder)
        .unwrap_or(&NOOP_FALLBACK)
}

#[cfg(not(any(feature = "telemetry-grpc", feature = "telemetry-http")))]
pub(crate) fn metrics() -> &'static dyn MetricsRecorder {
    RECORDER.get_or_init(noop::NoOpRecorder::new)
}

/// Set the global recorder (called during initialization)
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
pub(crate) fn set_recorder(recorder: otel_recorder::OtelRecorder) {
    let _ = RECORDER.set(recorder);
}

// No-telemetry stubs (actually used in no-telemetry builds)
#[cfg(not(any(feature = "telemetry-grpc", feature = "telemetry-http")))]
pub fn spawn_instrumented<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(future)
}

#[cfg(not(any(feature = "telemetry-grpc", feature = "telemetry-http")))]
pub async fn sample_app_list_metrics(_app_state: crate::app_state::SharedAppState) {}

#[cfg(not(any(feature = "telemetry-grpc", feature = "telemetry-http")))]
pub async fn sample_memory_metrics() {}

#[cfg(not(any(feature = "telemetry-grpc", feature = "telemetry-http")))]
pub async fn sample_tokio_metrics() {}

// OAuth metrics helpers
pub fn record_oauth_sessions_expired_cleaned(count: usize) {
    metrics().record_oauth_sessions_expired_cleaned(count);
}

pub fn record_oauth_session_counts(device_count: usize, web_count: usize, _session_count: usize) {
    let m = metrics();
    m.record_oauth_device_sessions(device_count as u64);
    m.record_oauth_web_sessions(web_count as u64);
}

// Module-style wrappers for existing call sites (both telemetry and no-telemetry)
pub mod websocket {
    #[inline]
    pub fn record_connection_opened() {
        super::metrics().record_websocket_connection_opened();
    }

    #[inline]
    pub fn record_connection_closed() {
        super::metrics().record_websocket_connection_closed();
    }

    #[inline]
    pub fn record_message_sent() {
        super::metrics().record_websocket_message_sent();
    }

    #[inline]
    pub fn record_messages_sent(count: usize) {
        if count > 0 {
            super::metrics().record_websocket_messages_sent(count);
        }
    }

    #[inline]
    pub fn record_message_received() {
        super::metrics().record_websocket_message_received();
    }

    #[inline]
    pub fn record_auth_failure() {
        super::metrics().record_websocket_auth_failure();
    }
}

pub mod shell {
    #[inline]
    pub fn record_session_started() {
        super::metrics().record_shell_session_started();
    }

    #[inline]
    pub fn record_session_ended(duration_secs: f64) {
        super::metrics().record_shell_session_ended(duration_secs);
    }

    #[inline]
    pub fn record_session_error(duration_secs: f64) {
        super::metrics().record_shell_session_error(duration_secs);
    }

    #[inline]
    pub fn record_session_timeout(duration_secs: f64) {
        super::metrics().record_shell_session_timeout(duration_secs);
    }
}

pub mod tasks {
    #[inline]
    pub fn record_stream_started() {
        // Not implemented in metrics recorder yet
    }

    #[inline]
    pub fn record_stream_ended() {
        // Not implemented in metrics recorder yet
    }

    #[inline]
    pub fn record_output_lines(_count: usize) {
        // Not implemented in metrics recorder yet
    }
}
