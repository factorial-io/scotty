//! OpenTelemetry metrics recorder wrapping ScottyMetrics

use super::instruments::ScottyMetrics;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Instant;

// Active counts tracked in memory
static ACTIVE_SHELL_SESSIONS: AtomicI64 = AtomicI64::new(0);
static ACTIVE_WEBSOCKET_CONNECTIONS: AtomicI64 = AtomicI64::new(0);

/// Wrapper around ScottyMetrics providing a clean API
pub(crate) struct OtelRecorder {
    pub(super) instruments: ScottyMetrics,
}

impl OtelRecorder {
    pub(crate) fn new(instruments: ScottyMetrics) -> Self {
        Self { instruments }
    }

    /// Get direct access to instruments (for internal metrics modules)
    #[allow(dead_code)]
    pub(super) fn instruments(&self) -> &ScottyMetrics {
        &self.instruments
    }

    // Log streaming
    pub(crate) fn record_log_stream_started(&self, active_count: usize) {
        self.instruments
            .log_streams_active
            .record(active_count as i64, &[]);
        self.instruments.log_streams_total.add(1, &[]);
    }

    pub(crate) fn record_log_stream_ended(&self, active_count: usize, duration_secs: f64) {
        self.instruments
            .log_streams_active
            .record(active_count as i64, &[]);
        self.instruments
            .log_stream_duration
            .record(duration_secs, &[]);
    }

    pub(crate) fn record_log_line_received(&self) {
        self.instruments.log_lines_received.add(1, &[]);
    }

    pub(crate) fn record_log_stream_error(&self) {
        self.instruments.log_stream_errors.add(1, &[]);
    }

    // Shell sessions
    pub(crate) fn record_shell_session_started(&self) {
        let count = ACTIVE_SHELL_SESSIONS.fetch_add(1, Ordering::Relaxed) + 1;
        self.instruments.shell_sessions_total.add(1, &[]);
        self.instruments.shell_sessions_active.record(count, &[]);
    }

    pub(crate) fn record_shell_session_ended(&self, duration_secs: f64) {
        let count = ACTIVE_SHELL_SESSIONS.fetch_sub(1, Ordering::Relaxed) - 1;
        self.instruments.shell_sessions_active.record(count, &[]);
        self.instruments
            .shell_session_duration
            .record(duration_secs, &[]);
    }

    pub(crate) fn record_shell_session_error(&self, duration_secs: f64) {
        let count = ACTIVE_SHELL_SESSIONS.fetch_sub(1, Ordering::Relaxed) - 1;
        self.instruments.shell_sessions_active.record(count, &[]);
        self.instruments.shell_session_errors.add(1, &[]);
        self.instruments
            .shell_session_duration
            .record(duration_secs, &[]);
    }

    pub(crate) fn record_shell_session_timeout(&self, duration_secs: f64) {
        let count = ACTIVE_SHELL_SESSIONS.fetch_sub(1, Ordering::Relaxed) - 1;
        self.instruments.shell_sessions_active.record(count, &[]);
        self.instruments.shell_session_timeouts.add(1, &[]);
        self.instruments
            .shell_session_duration
            .record(duration_secs, &[]);
    }

    // WebSocket
    pub(crate) fn record_websocket_connection_opened(&self) {
        let count = ACTIVE_WEBSOCKET_CONNECTIONS.fetch_add(1, Ordering::Relaxed) + 1;
        self.instruments
            .websocket_connections_active
            .record(count, &[]);
    }

    pub(crate) fn record_websocket_connection_closed(&self) {
        let count = ACTIVE_WEBSOCKET_CONNECTIONS.fetch_sub(1, Ordering::Relaxed) - 1;
        self.instruments
            .websocket_connections_active
            .record(count, &[]);
    }

    pub(crate) fn record_websocket_message_received(&self) {
        self.instruments.websocket_messages_received.add(1, &[]);
    }

    pub(crate) fn record_websocket_message_sent(&self) {
        self.instruments.websocket_messages_sent.add(1, &[]);
    }

    pub(crate) fn record_websocket_messages_sent(&self, count: usize) {
        self.instruments
            .websocket_messages_sent
            .add(count as u64, &[]);
    }

    pub(crate) fn record_websocket_auth_failure(&self) {
        self.instruments.websocket_auth_failures.add(1, &[]);
    }

    // Tasks
    pub(crate) fn record_task_added(&self, active_count: usize) {
        self.instruments
            .tasks_active
            .record(active_count as i64, &[]);
        self.instruments.tasks_total.add(1, &[]);
    }

    pub(crate) fn record_task_finished(&self, duration_secs: f64, failed: bool) {
        self.instruments.task_duration.record(duration_secs, &[]);
        if failed {
            self.instruments.task_failures.add(1, &[]);
        }
    }

    pub(crate) fn record_task_cleanup(&self, active_count: usize) {
        self.instruments
            .tasks_active
            .record(active_count as i64, &[]);
    }

    // HTTP
    #[allow(dead_code)]
    pub(crate) fn record_http_request_started(&self) {
        self.instruments.http_requests_active.add(1, &[]);
    }

    #[allow(dead_code)]
    pub(crate) fn record_http_request_finished(&self, duration_secs: f64, status: u16) {
        use opentelemetry::KeyValue;
        let labels = [KeyValue::new("status", status.to_string())];
        self.instruments.http_requests_total.add(1, &labels);
        self.instruments
            .http_request_duration
            .record(duration_secs, &labels);
        self.instruments.http_requests_active.add(-1, &[]);
    }

    // OAuth
    pub(crate) fn record_oauth_device_flow_start(&self) {
        self.instruments.oauth_device_flows_total.add(1, &[]);
    }

    pub(crate) fn record_oauth_web_flow_start(&self) {
        self.instruments.oauth_web_flows_total.add(1, &[]);
    }

    pub(crate) fn record_oauth_flow_failure(&self) {
        self.instruments.oauth_flow_failures.add(1, &[]);
    }

    pub(crate) fn record_oauth_token_validation(&self, start: Instant, failed: bool) {
        let duration = start.elapsed().as_secs_f64();
        self.instruments.oauth_token_validations_total.add(1, &[]);
        self.instruments
            .oauth_token_validation_duration
            .record(duration, &[]);
        if failed {
            self.instruments.oauth_token_validation_failures.add(1, &[]);
        }
    }

    // Rate limiting
    pub(crate) fn record_rate_limit_allowed(&self, tier: &str) {
        use opentelemetry::KeyValue;
        let labels = [
            KeyValue::new("tier", tier.to_string()),
            KeyValue::new("decision", "allowed"),
        ];
        self.instruments.rate_limit_requests_total.add(1, &labels);
    }

    pub(crate) fn record_rate_limit_denied(&self, tier: &str) {
        use opentelemetry::KeyValue;
        let labels = [
            KeyValue::new("tier", tier.to_string()),
            KeyValue::new("decision", "denied"),
        ];
        self.instruments.rate_limit_requests_total.add(1, &labels);
    }

    pub(crate) fn record_rate_limit_extractor_error(&self) {
        self.instruments.rate_limit_extractor_errors.add(1, &[]);
    }
}
