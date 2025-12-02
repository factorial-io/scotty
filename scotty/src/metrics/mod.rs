//! OpenTelemetry metrics for Scotty
//!
//! This module provides metrics instrumentation for the unified output system.

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

#[cfg(feature = "no-telemetry")]
mod noop;

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

#[cfg(feature = "no-telemetry")]
static RECORDER: OnceLock<noop::NoOpRecorder> = OnceLock::new();

/// Get the global metrics recorder
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
pub(crate) fn metrics() -> &'static otel_recorder::OtelRecorder {
    RECORDER.get().expect("Metrics not initialized")
}

#[cfg(feature = "no-telemetry")]
pub(crate) fn metrics() -> &'static noop::NoOpRecorder {
    RECORDER.get_or_init(|| noop::NoOpRecorder::new())
}

/// Get direct access to instruments (for internal metrics modules only)
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
pub(crate) fn get_metrics() -> Option<&'static instruments::ScottyMetrics> {
    RECORDER.get().map(|r| &r.instruments)
}

/// Set the global recorder (called during initialization)
#[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
pub(crate) fn set_recorder(recorder: otel_recorder::OtelRecorder) {
    let _ = RECORDER.set(recorder);
}

// No-telemetry stubs for functions called during setup
#[cfg(feature = "no-telemetry")]
pub fn init_metrics() -> anyhow::Result<()> {
    Ok(())
}

#[cfg(feature = "no-telemetry")]
pub async fn http_metrics_middleware(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    next.run(request).await
}

#[cfg(feature = "no-telemetry")]
pub fn spawn_instrumented<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(future)
}

#[cfg(feature = "no-telemetry")]
pub async fn sample_app_list_metrics(_app_state: crate::app_state::SharedAppState) {}

#[cfg(feature = "no-telemetry")]
pub async fn sample_memory_metrics() {}

#[cfg(feature = "no-telemetry")]
pub async fn sample_tokio_metrics() {}

// Module-style wrappers for existing call sites (both telemetry and no-telemetry)
pub mod websocket {
    use std::sync::atomic::{AtomicI64, Ordering};
    static ACTIVE_CONNECTIONS: AtomicI64 = AtomicI64::new(0);

    #[inline]
    pub fn record_connection_opened() {
        let count = ACTIVE_CONNECTIONS.fetch_add(1, Ordering::Relaxed) + 1;
        super::metrics().record_websocket_connection_opened();
        // Note: active count tracking done via atomic
        let _ = count; // For future use
    }

    #[inline]
    pub fn record_connection_closed() {
        let _count = ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::Relaxed) - 1;
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
