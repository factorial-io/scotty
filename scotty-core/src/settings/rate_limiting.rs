use serde::Deserialize;

/// Rate limiting configuration for the API
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RateLimitingConfig {
    /// Global enable/disable switch for all rate limiting
    #[serde(default)]
    pub enabled: bool,

    /// Rate limits for public authentication endpoints (login)
    /// Rate limited by IP address
    #[serde(default)]
    pub public_auth: TierConfig,

    /// Rate limits for OAuth endpoints
    /// Rate limited by IP address
    #[serde(default)]
    pub oauth: TierConfig,

    /// Rate limits for authenticated API endpoints
    /// Rate limited by bearer token
    #[serde(default)]
    pub authenticated: TierConfig,
}

/// Configuration for a single rate limiting tier
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TierConfig {
    /// Maximum requests per minute
    #[serde(default)]
    pub requests_per_minute: u64,

    /// Burst size (maximum requests in a short burst)
    #[serde(default)]
    pub burst_size: u32,
}

impl TierConfig {
    /// Check if this tier is enabled (non-zero rate limit)
    pub fn is_enabled(&self) -> bool {
        self.requests_per_minute > 0
    }
}
