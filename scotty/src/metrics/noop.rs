//! No-op metrics implementation for no-telemetry builds

use super::recorder_trait::MetricsRecorder;
use std::time::Instant;

/// Zero-cost no-op recorder
pub(crate) struct NoOpRecorder;

impl NoOpRecorder {
    pub(crate) const fn new() -> Self {
        Self
    }
}

// Trait implementation with zero-cost inline methods
impl MetricsRecorder for NoOpRecorder {
    #[inline(always)]
    fn record_log_stream_started(&self, _active_count: usize) {}

    #[inline(always)]
    fn record_log_stream_ended(&self, _active_count: usize, _duration_secs: f64) {}

    #[inline(always)]
    fn record_log_line_received(&self) {}

    #[inline(always)]
    fn record_log_stream_error(&self) {}

    #[inline(always)]
    fn record_shell_session_started(&self) {}

    #[inline(always)]
    fn record_shell_session_ended(&self, _duration_secs: f64) {}

    #[inline(always)]
    fn record_shell_session_error(&self, _duration_secs: f64) {}

    #[inline(always)]
    fn record_shell_session_timeout(&self, _duration_secs: f64) {}

    #[inline(always)]
    fn record_websocket_connection_opened(&self) {}

    #[inline(always)]
    fn record_websocket_connection_closed(&self) {}

    #[inline(always)]
    fn record_websocket_message_received(&self) {}

    #[inline(always)]
    fn record_websocket_message_sent(&self) {}

    #[inline(always)]
    fn record_websocket_messages_sent(&self, _count: usize) {}

    #[inline(always)]
    fn record_websocket_auth_failure(&self) {}

    #[inline(always)]
    fn record_task_added(&self, _active_count: usize) {}

    #[inline(always)]
    fn record_task_finished(&self, _duration_secs: f64, _failed: bool) {}

    #[inline(always)]
    fn record_task_cleanup(&self, _active_count: usize) {}

    #[inline(always)]
    fn record_oauth_device_flow_start(&self) {}

    #[inline(always)]
    fn record_oauth_web_flow_start(&self) {}

    #[inline(always)]
    fn record_oauth_flow_failure(&self) {}

    #[inline(always)]
    fn record_oauth_token_validation(&self, _start: Instant, _failed: bool) {}

    #[inline(always)]
    fn record_rate_limit_allowed(&self, _tier: &str) {}

    #[inline(always)]
    fn record_rate_limit_denied(&self, _tier: &str) {}

    #[inline(always)]
    fn record_rate_limit_extractor_error(&self) {}
}
