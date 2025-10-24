use opentelemetry::metrics::{Counter, Gauge, Histogram, Meter};

/// Scotty metrics instruments
///
/// Holds all OpenTelemetry metric instruments for monitoring
/// the unified output system (log streaming, shell sessions, WebSocket, tasks).
#[derive(Clone)]
pub struct ScottyMetrics {
    // Log streaming metrics
    pub log_streams_active: Gauge<i64>,
    pub log_streams_total: Counter<u64>,
    pub log_stream_duration: Histogram<f64>,
    pub log_lines_received: Counter<u64>,
    pub log_stream_errors: Counter<u64>,

    // Shell session metrics
    pub shell_sessions_active: Gauge<i64>,
    pub shell_sessions_total: Counter<u64>,
    pub shell_session_duration: Histogram<f64>,
    pub shell_session_errors: Counter<u64>,

    // WebSocket metrics
    pub websocket_connections_active: Gauge<i64>,
    pub websocket_messages_sent: Counter<u64>,

    // Task metrics
    pub tasks_active: Gauge<i64>,
    pub tasks_total: Counter<u64>,
}

impl ScottyMetrics {
    /// Create new metrics instance from a Meter
    pub fn new(meter: Meter) -> Self {
        Self {
            // Log streaming
            log_streams_active: meter
                .i64_gauge("scotty.log_streams.active")
                .with_description("Active log streams")
                .build(),

            log_streams_total: meter
                .u64_counter("scotty.log_streams.total")
                .with_description("Total log streams")
                .build(),

            log_stream_duration: meter
                .f64_histogram("scotty.log_stream.duration")
                .with_description("Log stream duration")
                .with_unit("s")
                .build(),

            log_lines_received: meter
                .u64_counter("scotty.log_stream.lines")
                .with_description("Log lines received")
                .build(),

            log_stream_errors: meter
                .u64_counter("scotty.log_stream.errors")
                .with_description("Log streaming errors")
                .build(),

            // Shell sessions
            shell_sessions_active: meter
                .i64_gauge("scotty.shell_sessions.active")
                .with_description("Active shell sessions")
                .build(),

            shell_sessions_total: meter
                .u64_counter("scotty.shell_sessions.total")
                .with_description("Total shell sessions")
                .build(),

            shell_session_duration: meter
                .f64_histogram("scotty.shell_session.duration")
                .with_description("Shell session duration")
                .with_unit("s")
                .build(),

            shell_session_errors: meter
                .u64_counter("scotty.shell_session.errors")
                .with_description("Shell session errors")
                .build(),

            // WebSocket
            websocket_connections_active: meter
                .i64_gauge("scotty.websocket.connections")
                .with_description("Active WebSocket connections")
                .build(),

            websocket_messages_sent: meter
                .u64_counter("scotty.websocket.messages_sent")
                .with_description("WebSocket messages sent")
                .build(),

            // Tasks
            tasks_active: meter
                .i64_gauge("scotty.tasks.active")
                .with_description("Active tasks")
                .build(),

            tasks_total: meter
                .u64_counter("scotty.tasks.total")
                .with_description("Total tasks executed")
                .build(),
        }
    }
}
