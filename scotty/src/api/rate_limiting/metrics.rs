//! Metrics recording for rate limiting

use crate::metrics;

/// Record a rate limit hit for a specific tier
pub fn record_rate_limit_hit(tier: &str) {
    if let Some(m) = metrics::get_metrics() {
        // Record total hits
        m.rate_limit_hits_total.add(1, &[]);

        // Record hits by tier
        m.rate_limit_hits_by_tier.add(
            1,
            &[opentelemetry::KeyValue::new("tier", tier.to_string())],
        );
    }
}
