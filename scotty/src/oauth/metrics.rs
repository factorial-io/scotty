use crate::metrics;
use std::time::Instant;

/// Record device flow start
pub fn record_device_flow_start() {
    if let Some(m) = metrics::get_metrics() {
        m.oauth_device_flows_total.add(1, &[]);
    }
}

/// Record web flow start
pub fn record_web_flow_start() {
    if let Some(m) = metrics::get_metrics() {
        m.oauth_web_flows_total.add(1, &[]);
    }
}

/// Record OAuth flow failure
#[allow(dead_code)] // Available for future use
pub fn record_flow_failure() {
    if let Some(m) = metrics::get_metrics() {
        m.oauth_flow_failures.add(1, &[]);
    }
}

/// Record token validation with timing
pub fn record_token_validation<T, E>(start: Instant, result: &Result<T, E>) {
    if let Some(m) = metrics::get_metrics() {
        let duration = start.elapsed().as_secs_f64();
        m.oauth_token_validations_total.add(1, &[]);
        m.oauth_token_validation_duration.record(duration, &[]);

        if result.is_err() {
            m.oauth_token_validation_failures.add(1, &[]);
        }
    }
}

/// Record token validation failure without timing
#[allow(dead_code)] // Available for future use
pub fn record_token_validation_failure() {
    if let Some(m) = metrics::get_metrics() {
        m.oauth_token_validations_total.add(1, &[]);
        m.oauth_token_validation_failures.add(1, &[]);
    }
}
