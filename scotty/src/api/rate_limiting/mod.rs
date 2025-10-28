//! Rate limiting for API endpoints
//!
//! This module provides tiered rate limiting for different types of endpoints:
//! - Public auth endpoints (login) - rate limited by IP
//! - OAuth endpoints - rate limited by IP
//! - Authenticated endpoints - rate limited by bearer token
//!
//! Rate limiting uses the token bucket algorithm via tower-governor.

pub mod config;
pub mod extractors;
pub mod metrics;
pub mod middleware;
#[cfg(test)]
mod tests;

pub use config::TierConfig;
use extractors::BearerTokenExtractor;

use governor::middleware::NoOpMiddleware;
use std::sync::Arc;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::GovernorLayer;

/// Create rate limiter for public auth endpoints (login)
///
/// Rate limits by IP address to prevent brute force attacks.
pub fn create_public_auth_limiter(
    config: &TierConfig,
) -> GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware, axum::body::Body> {
    create_ip_limiter(config)
}

/// Create rate limiter for OAuth endpoints
///
/// Rate limits by IP address to prevent DoS and session exhaustion attacks.
pub fn create_oauth_limiter(
    config: &TierConfig,
) -> GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware, axum::body::Body> {
    create_ip_limiter(config)
}

/// Create rate limiter for authenticated API endpoints
///
/// Rate limits by bearer token to prevent per-user abuse.
pub fn create_authenticated_limiter(
    config: &TierConfig,
) -> GovernorLayer<BearerTokenExtractor, NoOpMiddleware, axum::body::Body> {
    // Calculate per_second rate, ensuring at least 1 per minute (avoid division by zero)
    let per_second = std::cmp::max(1, config.requests_per_minute / 60);

    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(per_second)
            .burst_size(config.burst_size)
            .key_extractor(BearerTokenExtractor)
            .finish()
            .expect("Invalid rate limit config"),
    );

    GovernorLayer::new(governor_config)
}

/// Helper to create IP-based rate limiters
fn create_ip_limiter(
    config: &TierConfig,
) -> GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware, axum::body::Body> {
    // Calculate per_second rate, ensuring at least 1 per minute (avoid division by zero)
    let per_second = std::cmp::max(1, config.requests_per_minute / 60);

    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(per_second)
            .burst_size(config.burst_size)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("Invalid rate limit config"),
    );

    GovernorLayer::new(governor_config)
}
