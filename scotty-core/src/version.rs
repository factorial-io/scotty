use anyhow::{Context, Result};
use semver::Version;

/// Version management utilities for Scotty workspace
pub struct VersionManager;

impl VersionManager {
    /// Parse a version string using semver
    pub fn parse_version(version_str: &str) -> Result<Version> {
        Version::parse(version_str).context("Failed to parse version")
    }

    /// Check if two versions are compatible (major.minor must match)
    pub fn are_compatible(client_version: &Version, server_version: &Version) -> bool {
        client_version.major == server_version.major && client_version.minor == server_version.minor
    }

    /// Get the current package version at compile time
    pub fn current_version() -> Result<Version> {
        let version_str = env!("CARGO_PKG_VERSION");
        Self::parse_version(version_str)
    }

    /// Compare two versions and return which should be updated
    pub fn get_update_recommendation(
        client_version: &Version,
        server_version: &Version,
    ) -> Option<UpdateRecommendation> {
        if Self::are_compatible(client_version, server_version) {
            None
        } else if client_version < server_version {
            Some(UpdateRecommendation::UpdateClient)
        } else {
            Some(UpdateRecommendation::UpdateServer)
        }
    }

    /// Format versions for user-friendly display
    pub fn format_version_comparison(client_version: &Version, server_version: &Version) -> String {
        format!(
            "Client: {} | Server: {}",
            Self::format_single_version(client_version),
            Self::format_single_version(server_version)
        )
    }

    /// Format a single version for display
    pub fn format_single_version(version: &Version) -> String {
        if version.pre.is_empty() {
            version.to_string()
        } else {
            format!("{} ({})", version, version.pre)
        }
    }

    /// Check if a version is a pre-release
    pub fn is_prerelease(version: &Version) -> bool {
        !version.pre.is_empty()
    }

    /// Get a user-friendly pre-release type name
    pub fn prerelease_type(version: &Version) -> String {
        if version.pre.is_empty() {
            "stable".to_string()
        } else {
            // Extract the pre-release identifier (alpha, beta, rc, etc.)
            version.pre.to_string()
        }
    }
}

/// Recommendation for which component should be updated
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateRecommendation {
    UpdateClient,
    UpdateServer,
}

impl std::fmt::Display for UpdateRecommendation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateRecommendation::UpdateClient => write!(f, "scottyctl"),
            UpdateRecommendation::UpdateServer => write!(f, "the scotty server"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compatibility() {
        let v1_0_0 = Version::parse("1.0.0").unwrap();
        let v1_0_1 = Version::parse("1.0.1").unwrap();
        let v1_1_0 = Version::parse("1.1.0").unwrap();
        let v2_0_0 = Version::parse("2.0.0").unwrap();

        // Same major.minor should be compatible
        assert!(VersionManager::are_compatible(&v1_0_0, &v1_0_1));
        assert!(VersionManager::are_compatible(&v1_0_1, &v1_0_0));

        // Different minor should not be compatible
        assert!(!VersionManager::are_compatible(&v1_0_0, &v1_1_0));

        // Different major should not be compatible
        assert!(!VersionManager::are_compatible(&v1_0_0, &v2_0_0));
    }

    #[test]
    fn test_update_recommendations() {
        let v1_0_0 = Version::parse("1.0.0").unwrap();
        let v1_0_1 = Version::parse("1.0.1").unwrap();
        let v1_1_0 = Version::parse("1.1.0").unwrap();

        // Compatible versions should have no recommendation
        assert_eq!(
            VersionManager::get_update_recommendation(&v1_0_0, &v1_0_1),
            None
        );

        // Client older than server
        assert_eq!(
            VersionManager::get_update_recommendation(&v1_0_0, &v1_1_0),
            Some(UpdateRecommendation::UpdateClient)
        );

        // Server older than client
        assert_eq!(
            VersionManager::get_update_recommendation(&v1_1_0, &v1_0_0),
            Some(UpdateRecommendation::UpdateServer)
        );
    }

    #[test]
    fn test_prerelease_detection() {
        let stable = Version::parse("1.0.0").unwrap();
        let alpha = Version::parse("1.0.0-alpha.1").unwrap();
        let beta = Version::parse("1.0.0-beta").unwrap();

        assert!(!VersionManager::is_prerelease(&stable));
        assert!(VersionManager::is_prerelease(&alpha));
        assert!(VersionManager::is_prerelease(&beta));

        assert_eq!(VersionManager::prerelease_type(&stable), "stable");
        assert_eq!(VersionManager::prerelease_type(&alpha), "alpha.1");
        assert_eq!(VersionManager::prerelease_type(&beta), "beta");
    }
}
