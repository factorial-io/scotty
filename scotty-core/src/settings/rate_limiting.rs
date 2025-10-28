use serde::Deserialize;

/// Rate limiting configuration validation error
#[derive(Debug)]
pub struct RateLimitingValidationError {
    pub message: String,
}

impl std::fmt::Display for RateLimitingValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rate limiting configuration error: {}", self.message)
    }
}

impl std::error::Error for RateLimitingValidationError {}

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

impl RateLimitingConfig {
    /// Validate the rate limiting configuration
    pub fn validate(&self) -> Result<(), RateLimitingValidationError> {
        if !self.enabled {
            return Ok(()); // Skip validation if rate limiting is disabled
        }

        // Validate each tier
        self.public_auth
            .validate()
            .map_err(|e| RateLimitingValidationError {
                message: format!("public_auth: {}", e.message),
            })?;

        self.oauth
            .validate()
            .map_err(|e| RateLimitingValidationError {
                message: format!("oauth: {}", e.message),
            })?;

        self.authenticated
            .validate()
            .map_err(|e| RateLimitingValidationError {
                message: format!("authenticated: {}", e.message),
            })?;

        Ok(())
    }
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

    /// Validate the tier configuration
    pub fn validate(&self) -> Result<(), RateLimitingValidationError> {
        // If disabled (requests_per_minute = 0), burst_size should also be 0
        if self.requests_per_minute == 0 && self.burst_size > 0 {
            return Err(RateLimitingValidationError {
                message: "burst_size must be 0 when requests_per_minute is 0".to_string(),
            });
        }

        // If enabled, burst_size must be greater than 0
        if self.requests_per_minute > 0 && self.burst_size == 0 {
            return Err(RateLimitingValidationError {
                message: "burst_size must be greater than 0 when rate limiting is enabled"
                    .to_string(),
            });
        }

        // Burst size should not exceed total requests per minute
        // (This is a warning condition, not a hard error, but we'll enforce it)
        if self.burst_size as u64 > self.requests_per_minute {
            return Err(RateLimitingValidationError {
                message: format!(
                    "burst_size ({}) should not exceed requests_per_minute ({})",
                    self.burst_size, self.requests_per_minute
                ),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_config_valid() {
        let config = TierConfig {
            requests_per_minute: 60,
            burst_size: 10,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_tier_config_disabled_valid() {
        let config = TierConfig {
            requests_per_minute: 0,
            burst_size: 0,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_tier_config_burst_without_rate_invalid() {
        let config = TierConfig {
            requests_per_minute: 0,
            burst_size: 10,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_tier_config_rate_without_burst_invalid() {
        let config = TierConfig {
            requests_per_minute: 60,
            burst_size: 0,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_tier_config_burst_exceeds_rate_invalid() {
        let config = TierConfig {
            requests_per_minute: 10,
            burst_size: 20,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_rate_limiting_config_disabled_skips_validation() {
        let config = RateLimitingConfig {
            enabled: false,
            public_auth: TierConfig {
                requests_per_minute: 0,
                burst_size: 999, // Invalid, but should be skipped
            },
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_rate_limiting_config_validates_all_tiers() {
        let config = RateLimitingConfig {
            enabled: true,
            public_auth: TierConfig {
                requests_per_minute: 60,
                burst_size: 10,
            },
            oauth: TierConfig {
                requests_per_minute: 300,
                burst_size: 50,
            },
            authenticated: TierConfig {
                requests_per_minute: 600,
                burst_size: 100,
            },
        };
        assert!(config.validate().is_ok());
    }
}
