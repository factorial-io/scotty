//! Metrics recording for rate limiting

use crate::metrics;

/// Record an allowed rate limit request
pub fn record_allowed_request(tier: &str) {
    metrics::metrics().record_rate_limit_allowed(tier);
}

/// Record a denied rate limit request (429 response)
pub fn record_denied_request(tier: &str) {
    metrics::metrics().record_rate_limit_denied(tier);
}

/// Record a key extraction error
pub fn record_extractor_error() {
    metrics::metrics().record_rate_limit_extractor_error();
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
