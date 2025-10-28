//! Metrics recording for rate limiting

use crate::metrics;

/// Record an allowed rate limit request
pub fn record_allowed_request(tier: &str) {
    if let Some(m) = metrics::get_metrics() {
        m.rate_limit_requests_total.add(
            1,
            &[
                opentelemetry::KeyValue::new("tier", tier.to_string()),
                opentelemetry::KeyValue::new("status", "allowed"),
            ],
        );
    }
}

/// Record a denied rate limit request (429 response)
pub fn record_denied_request(tier: &str) {
    if let Some(m) = metrics::get_metrics() {
        m.rate_limit_requests_total.add(
            1,
            &[
                opentelemetry::KeyValue::new("tier", tier.to_string()),
                opentelemetry::KeyValue::new("status", "denied"),
            ],
        );
    }
}

/// Record a key extraction error
pub fn record_extractor_error() {
    if let Some(m) = metrics::get_metrics() {
        m.rate_limit_extractor_errors.add(1, &[]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording_does_not_panic() {
        // These functions should not panic even when metrics aren't initialized
        record_allowed_request("test_tier");
        record_denied_request("test_tier");
        record_extractor_error();
    }
}
