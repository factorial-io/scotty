use opentelemetry::metrics::{Counter, Gauge, Histogram, Meter, UpDownCounter};

/// Scotty metrics instruments
///
/// Holds all OpenTelemetry metric instruments for monitoring
/// the unified output system (log streaming, shell sessions, WebSocket, tasks).
#[derive(Clone)]
#[allow(dead_code)] // Some metrics are not yet instrumented (shell, websocket)
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
    pub task_duration: Histogram<f64>,
    pub task_failures: Counter<u64>,

    // Memory metrics
    pub memory_rss_bytes: Gauge<u64>,
    pub memory_virtual_bytes: Gauge<u64>,

    // Tokio runtime metrics
    pub tokio_workers_count: Gauge<u64>,
    pub tokio_active_tasks_count: Gauge<u64>,
    pub tokio_tasks_dropped: Counter<u64>,
    pub tokio_poll_count: Counter<u64>,
    pub tokio_poll_duration: Histogram<f64>,
    pub tokio_slow_poll_count: Counter<u64>,
    pub tokio_idle_duration: Histogram<f64>,
    pub tokio_scheduled_count: Counter<u64>,
    pub tokio_first_poll_delay: Histogram<f64>,

    // HTTP server metrics
    pub http_requests_total: Counter<u64>,
    pub http_request_duration: Histogram<f64>,
    pub http_requests_active: UpDownCounter<i64>,
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

            task_duration: meter
                .f64_histogram("scotty.task.duration")
                .with_description("Task execution duration")
                .with_unit("s")
                .build(),

            task_failures: meter
                .u64_counter("scotty.task.failures")
                .with_description("Failed tasks")
                .build(),

            // Memory
            memory_rss_bytes: meter
                .u64_gauge("scotty.memory.rss_bytes")
                .with_description("Resident Set Size (RSS) in bytes")
                .with_unit("bytes")
                .build(),

            memory_virtual_bytes: meter
                .u64_gauge("scotty.memory.virtual_bytes")
                .with_description("Virtual memory size in bytes")
                .with_unit("bytes")
                .build(),

            // Tokio runtime
            tokio_workers_count: meter
                .u64_gauge("scotty.tokio.workers.count")
                .with_description("Number of Tokio worker threads")
                .build(),

            tokio_active_tasks_count: meter
                .u64_gauge("scotty.tokio.tasks.active")
                .with_description("Number of active instrumented tasks")
                .build(),

            tokio_tasks_dropped: meter
                .u64_counter("scotty.tokio.tasks.dropped")
                .with_description("Number of completed tasks")
                .build(),

            tokio_poll_count: meter
                .u64_counter("scotty.tokio.poll.count")
                .with_description("Total number of task polls")
                .build(),

            tokio_poll_duration: meter
                .f64_histogram("scotty.tokio.poll.duration")
                .with_description("Task poll duration")
                .with_unit("s")
                .build(),

            tokio_slow_poll_count: meter
                .u64_counter("scotty.tokio.poll.slow_count")
                .with_description("Number of slow task polls")
                .build(),

            tokio_idle_duration: meter
                .f64_histogram("scotty.tokio.idle.duration")
                .with_description("Task idle duration between polls")
                .with_unit("s")
                .build(),

            tokio_scheduled_count: meter
                .u64_counter("scotty.tokio.scheduled.count")
                .with_description("Number of times tasks were scheduled")
                .build(),

            tokio_first_poll_delay: meter
                .f64_histogram("scotty.tokio.first_poll.delay")
                .with_description("Delay between task creation and first poll")
                .with_unit("s")
                .build(),

            // HTTP server
            http_requests_total: meter
                .u64_counter("scotty.http.requests.total")
                .with_description("Total HTTP requests")
                .build(),

            http_request_duration: meter
                .f64_histogram("scotty.http.request.duration")
                .with_description("HTTP request duration")
                .with_unit("s")
                .build(),

            http_requests_active: meter
                .i64_up_down_counter("scotty.http.requests.active")
                .with_description("Active HTTP requests")
                .build(),
        }
    }
}
