use axum::{extract::Request, middleware::Next, response::Response};
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
    super::metrics().record_http_requests_active_increment();

    // Process the request
    let response = next.run(request).await;

    // Record metrics after request completes
    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    super::metrics().record_http_request_finished(&method, &path, &status, duration);
    super::metrics().record_http_requests_active_decrement();

    response
}
