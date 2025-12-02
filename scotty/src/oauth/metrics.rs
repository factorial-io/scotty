use crate::metrics;
use std::time::Instant;

/// Record device flow start
pub fn record_device_flow_start() {
    metrics::metrics().record_oauth_device_flow_start();
}

/// Record web flow start
pub fn record_web_flow_start() {
    metrics::metrics().record_oauth_web_flow_start();
}

/// Record OAuth flow failure
#[allow(dead_code)] // Available for future use
pub fn record_flow_failure() {
    metrics::metrics().record_oauth_flow_failure();
}

/// Record token validation with timing
pub fn record_token_validation<T, E>(start: Instant, result: &Result<T, E>) {
    metrics::metrics().record_oauth_token_validation(start, result.is_err());
}

/// Record token validation failure without timing
#[allow(dead_code)] // Available for future use
pub fn record_token_validation_failure() {
    metrics::metrics().record_oauth_token_validation(Instant::now(), true);
}
