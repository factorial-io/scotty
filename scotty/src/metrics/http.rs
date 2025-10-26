use axum::{extract::Request, middleware::Next, response::Response};
use opentelemetry::KeyValue;
use std::time::Instant;

/// HTTP metrics middleware that tracks request counts, durations, and active requests
///
/// Metrics exposed:
/// - `scotty_http_requests_total` (Counter): Total HTTP requests by method, route, and status
/// - `scotty_http_request_duration_seconds` (Histogram): Request duration in seconds
/// - `scotty_http_requests_active` (UpDownCounter): Current number of active requests
pub async fn http_metrics_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    // Increment active requests
    if let Some(m) = super::get_metrics() {
        m.http_requests_active.add(1, &[]);
    }

    // Process the request
    let response = next.run(request).await;

    // Record metrics after request completes
    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    if let Some(m) = super::get_metrics() {
        let labels = [
            KeyValue::new("method", method),
            KeyValue::new("route", path),
            KeyValue::new("status", status),
        ];

        m.http_requests_total.add(1, &labels);
        m.http_request_duration.record(duration, &labels);
        m.http_requests_active.add(-1, &[]);
    }

    response
}
