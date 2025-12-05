//! OpenTelemetry metrics recorder wrapping ScottyMetrics

use super::instruments::ScottyMetrics;
use super::recorder_trait::MetricsRecorder;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Instant;

/// Wrapper around ScottyMetrics providing a clean API
///
/// Active counts are tracked within each recorder instance to support proper
/// test isolation and avoid shared state issues when metrics are re-initialized.
pub(crate) struct OtelRecorder {
    pub(super) instruments: ScottyMetrics,
    /// Active shell sessions count (used to calculate gauge values)
    active_shell_sessions: AtomicI64,
    /// Active WebSocket connections count (used to calculate gauge values)
    active_websocket_connections: AtomicI64,
}

impl OtelRecorder {
    pub(crate) fn new(instruments: ScottyMetrics) -> Self {
        Self {
            instruments,
            active_shell_sessions: AtomicI64::new(0),
            active_websocket_connections: AtomicI64::new(0),
        }
    }

    /// Get direct access to instruments (for internal metrics modules)
    #[allow(dead_code)]
    pub(super) fn instruments(&self) -> &ScottyMetrics {
        &self.instruments
    }

    /// HTTP request finished (not in trait - used by middleware)
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
}

// Trait implementation - the actual recording logic
impl MetricsRecorder for OtelRecorder {
    // Log streaming
    fn record_log_stream_started(&self, active_count: usize) {
        self.instruments
            .log_streams_active
            .record(active_count as i64, &[]);
        self.instruments.log_streams_total.add(1, &[]);
    }

    fn record_log_stream_ended(&self, active_count: usize, duration_secs: f64) {
        self.instruments
            .log_streams_active
            .record(active_count as i64, &[]);
        self.instruments
            .log_stream_duration
            .record(duration_secs, &[]);
    }

    fn record_log_line_received(&self) {
        self.instruments.log_lines_received.add(1, &[]);
    }

    fn record_log_stream_error(&self) {
        self.instruments.log_stream_errors.add(1, &[]);
    }

    // Shell sessions
    fn record_shell_session_started(&self) {
        let count = self.active_shell_sessions.fetch_add(1, Ordering::Relaxed) + 1;
        self.instruments.shell_sessions_total.add(1, &[]);
        self.instruments.shell_sessions_active.record(count, &[]);
    }

    fn record_shell_session_ended(&self, duration_secs: f64) {
        let count = self.active_shell_sessions.fetch_sub(1, Ordering::Relaxed) - 1;
        self.instruments.shell_sessions_active.record(count, &[]);
        self.instruments
            .shell_session_duration
            .record(duration_secs, &[]);
    }

    fn record_shell_session_error(&self, duration_secs: f64) {
        let count = self.active_shell_sessions.fetch_sub(1, Ordering::Relaxed) - 1;
        self.instruments.shell_sessions_active.record(count, &[]);
        self.instruments.shell_session_errors.add(1, &[]);
        self.instruments
            .shell_session_duration
            .record(duration_secs, &[]);
    }

    fn record_shell_session_timeout(&self, duration_secs: f64) {
        let count = self.active_shell_sessions.fetch_sub(1, Ordering::Relaxed) - 1;
        self.instruments.shell_sessions_active.record(count, &[]);
        self.instruments.shell_session_timeouts.add(1, &[]);
        self.instruments
            .shell_session_duration
            .record(duration_secs, &[]);
    }

    // WebSocket
    fn record_websocket_connection_opened(&self) {
        let count = self
            .active_websocket_connections
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        self.instruments
            .websocket_connections_active
            .record(count, &[]);
    }

    fn record_websocket_connection_closed(&self) {
        let count = self
            .active_websocket_connections
            .fetch_sub(1, Ordering::Relaxed)
            - 1;
        self.instruments
            .websocket_connections_active
            .record(count, &[]);
    }

    fn record_websocket_message_received(&self) {
        self.instruments.websocket_messages_received.add(1, &[]);
    }

    fn record_websocket_message_sent(&self) {
        self.instruments.websocket_messages_sent.add(1, &[]);
    }

    fn record_websocket_messages_sent(&self, count: usize) {
        self.instruments
            .websocket_messages_sent
            .add(count as u64, &[]);
    }

    fn record_websocket_auth_failure(&self) {
        self.instruments.websocket_auth_failures.add(1, &[]);
    }

    // Tasks
    fn record_task_added(&self, active_count: usize) {
        self.instruments
            .tasks_active
            .record(active_count as i64, &[]);
        self.instruments.tasks_total.add(1, &[]);
    }

    fn record_task_finished(&self, duration_secs: f64, failed: bool) {
        self.instruments.task_duration.record(duration_secs, &[]);
        if failed {
            self.instruments.task_failures.add(1, &[]);
        }
    }

    fn record_task_cleanup(&self, active_count: usize) {
        self.instruments
            .tasks_active
            .record(active_count as i64, &[]);
    }

    fn record_task_output_stream_started(&self) {
        // Task output streams are tracked separately from task execution
        // This could be extended to track active streams if needed
    }

    fn record_task_output_stream_ended(&self) {
        // Task output streams are tracked separately from task execution
        // This could be extended to track active streams if needed
    }

    fn record_task_output_lines(&self, count: usize) {
        self.instruments.task_output_lines.add(count as u64, &[]);
    }

    // OAuth
    fn record_oauth_device_flow_start(&self) {
        self.instruments.oauth_device_flows_total.add(1, &[]);
    }

    fn record_oauth_web_flow_start(&self) {
        self.instruments.oauth_web_flows_total.add(1, &[]);
    }

    fn record_oauth_flow_failure(&self) {
        self.instruments.oauth_flow_failures.add(1, &[]);
    }

    fn record_oauth_token_validation(&self, start: Instant, failed: bool) {
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
    fn record_rate_limit_allowed(&self, tier: &str) {
        use opentelemetry::KeyValue;
        let labels = [
            KeyValue::new("tier", tier.to_string()),
            KeyValue::new("decision", "allowed"),
        ];
        self.instruments.rate_limit_requests_total.add(1, &labels);
    }

    fn record_rate_limit_denied(&self, tier: &str) {
        use opentelemetry::KeyValue;
        let labels = [
            KeyValue::new("tier", tier.to_string()),
            KeyValue::new("decision", "denied"),
        ];
        self.instruments.rate_limit_requests_total.add(1, &labels);
    }

    fn record_rate_limit_extractor_error(&self) {
        self.instruments.rate_limit_extractor_errors.add(1, &[]);
    }

    // HTTP metrics
    fn record_http_requests_active_increment(&self) {
        self.instruments.http_requests_active.add(1, &[]);
    }

    fn record_http_requests_active_decrement(&self) {
        self.instruments.http_requests_active.add(-1, &[]);
    }

    fn record_http_request_finished(
        &self,
        method: &str,
        path: &str,
        status: &str,
        duration_secs: f64,
    ) {
        use opentelemetry::KeyValue;
        let labels = [
            KeyValue::new("method", method.to_string()),
            KeyValue::new("route", path.to_string()),
            KeyValue::new("status", status.to_string()),
        ];
        self.instruments.http_requests_total.add(1, &labels);
        self.instruments
            .http_request_duration
            .record(duration_secs, &labels);
    }

    // App list metrics
    fn record_apps_total(&self, count: u64) {
        self.instruments.apps_total.record(count, &[]);
    }

    fn record_apps_by_status(&self, status: &str, count: u64) {
        use opentelemetry::KeyValue;
        let labels = [KeyValue::new("status", status.to_string())];
        self.instruments.apps_by_status.record(count, &labels);
    }

    fn record_app_services_count(&self, count: f64) {
        self.instruments.app_services_count.record(count, &[]);
    }

    fn record_app_last_check_age(&self, seconds: f64) {
        self.instruments
            .app_last_check_age_seconds
            .record(seconds, &[]);
    }

    // Memory metrics
    fn record_memory_rss_bytes(&self, bytes: u64) {
        self.instruments.memory_rss_bytes.record(bytes, &[]);
    }

    fn record_memory_virtual_bytes(&self, bytes: u64) {
        self.instruments.memory_virtual_bytes.record(bytes, &[]);
    }

    // Tokio runtime metrics
    fn record_tokio_active_tasks(&self, count: u64) {
        self.instruments.tokio_active_tasks_count.record(count, &[]);
    }

    fn record_tokio_tasks_dropped(&self, count: u64) {
        self.instruments.tokio_tasks_dropped.add(count, &[]);
    }

    fn record_tokio_workers_count(&self, count: u64) {
        self.instruments.tokio_workers_count.record(count, &[]);
    }

    fn record_tokio_poll_count(&self, count: u64) {
        self.instruments.tokio_poll_count.add(count, &[]);
    }

    fn record_tokio_slow_poll_count(&self, count: u64) {
        self.instruments.tokio_slow_poll_count.add(count, &[]);
    }

    fn record_tokio_poll_duration(&self, duration_secs: f64) {
        self.instruments
            .tokio_poll_duration
            .record(duration_secs, &[]);
    }

    fn record_tokio_idle_duration(&self, duration_secs: f64) {
        self.instruments
            .tokio_idle_duration
            .record(duration_secs, &[]);
    }

    fn record_tokio_scheduled_count(&self, count: u64) {
        self.instruments.tokio_scheduled_count.add(count, &[]);
    }

    fn record_tokio_first_poll_delay(&self, duration_secs: f64) {
        self.instruments
            .tokio_first_poll_delay
            .record(duration_secs, &[]);
    }

    // OAuth session metrics
    fn record_oauth_device_sessions(&self, count: u64) {
        self.instruments
            .oauth_device_flow_sessions_active
            .record(count as i64, &[]);
    }

    fn record_oauth_web_sessions(&self, count: u64) {
        self.instruments
            .oauth_web_flow_sessions_active
            .record(count as i64, &[]);
    }

    fn record_oauth_sessions_expired_cleaned(&self, count: usize) {
        self.instruments
            .oauth_sessions_expired_cleaned
            .add(count as u64, &[]);
    }
}
