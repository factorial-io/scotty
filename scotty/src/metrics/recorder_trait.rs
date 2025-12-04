//! Common trait for metrics recorders

use std::time::Instant;

/// Metrics recorder trait
///
/// Implemented by both OtelRecorder and NoOpRecorder
pub trait MetricsRecorder: Send + Sync {
    fn record_log_stream_started(&self, active_count: usize);
    fn record_log_stream_ended(&self, active_count: usize, duration_secs: f64);
    fn record_log_line_received(&self);
    fn record_log_stream_error(&self);

    fn record_shell_session_started(&self);
    fn record_shell_session_ended(&self, duration_secs: f64);
    fn record_shell_session_error(&self, duration_secs: f64);
    fn record_shell_session_timeout(&self, duration_secs: f64);

    fn record_websocket_connection_opened(&self);
    fn record_websocket_connection_closed(&self);
    fn record_websocket_message_received(&self);
    fn record_websocket_message_sent(&self);
    fn record_websocket_messages_sent(&self, count: usize);
    fn record_websocket_auth_failure(&self);

    fn record_task_added(&self, active_count: usize);
    fn record_task_finished(&self, duration_secs: f64, failed: bool);
    fn record_task_cleanup(&self, active_count: usize);

    fn record_oauth_device_flow_start(&self);
    fn record_oauth_web_flow_start(&self);
    fn record_oauth_flow_failure(&self);
    fn record_oauth_token_validation(&self, start: Instant, failed: bool);

    fn record_rate_limit_allowed(&self, tier: &str);
    fn record_rate_limit_denied(&self, tier: &str);
    fn record_rate_limit_extractor_error(&self);

    // HTTP metrics
    fn record_http_requests_active_increment(&self);
    fn record_http_requests_active_decrement(&self);
    fn record_http_request_finished(
        &self,
        method: &str,
        path: &str,
        status: &str,
        duration_secs: f64,
    );

    // App list metrics
    fn record_apps_total(&self, count: u64);
    fn record_apps_by_status(&self, status: &str, count: u64);
    fn record_app_services_count(&self, count: f64);
    fn record_app_last_check_age(&self, seconds: f64);

    // Memory metrics
    fn record_memory_rss_bytes(&self, bytes: u64);
    fn record_memory_virtual_bytes(&self, bytes: u64);

    // Tokio runtime metrics
    fn record_tokio_active_tasks(&self, count: u64);
    fn record_tokio_tasks_dropped(&self, count: u64);
    fn record_tokio_workers_count(&self, count: u64);
    fn record_tokio_poll_count(&self, count: u64);
    fn record_tokio_slow_poll_count(&self, count: u64);
    fn record_tokio_poll_duration(&self, duration_secs: f64);
    fn record_tokio_idle_duration(&self, duration_secs: f64);
    fn record_tokio_scheduled_count(&self, count: u64);
    fn record_tokio_first_poll_delay(&self, duration_secs: f64);

    // OAuth session metrics
    fn record_oauth_device_sessions(&self, count: u64);
    fn record_oauth_web_sessions(&self, count: u64);
    fn record_oauth_sessions_expired_cleaned(&self, count: usize);
}
