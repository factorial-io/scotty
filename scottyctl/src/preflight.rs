use anyhow::{Context, Result};
use semver::Version;
use tracing::{debug, info, warn};

use crate::context::ServerSettings;
use crate::utils::ui::Ui;
use owo_colors::OwoColorize;
use scotty_core::api::ServerInfo;
use scotty_core::settings::api_server::AuthMode;
use std::sync::Arc;

pub struct PreflightChecker {
    server: ServerSettings,
    ui: Arc<Ui>,
}

impl PreflightChecker {
    pub fn new(server: ServerSettings, ui: Arc<Ui>) -> Self {
        Self { server, ui }
    }

    pub async fn check_compatibility(&self, bypass_check: bool) -> Result<()> {
        if bypass_check {
            warn!("Bypassing version compatibility check");
            self.ui.eprintln(
                "⚠️  Version compatibility check bypassed"
                    .yellow()
                    .to_string(),
            );
            return Ok(());
        }

        info!("Running preflight checks...");
        debug!("Checking version compatibility with server");

        let client_version = env!("CARGO_PKG_VERSION");
        let client_version = Version::parse(client_version)
            .context("Failed to parse client version")?;

        let server_info = match self.get_server_info().await {
            Ok(info) => info,
            Err(e) => {
                self.ui.eprintln(
                    format!("⚠️  Could not connect to server for version check: {}", e)
                        .yellow()
                        .to_string(),
                );
                return Err(e);
            }
        };

        let server_version = Version::parse(&server_info.version)
            .context("Failed to parse server version")?;

        debug!(
            "Version check - Client: {}, Server: {}",
            client_version, server_version
        );

        if !self.are_versions_compatible(&client_version, &server_version) {
            let error_msg = format!(
                "Version incompatibility detected!\n\
                 Client version: {} (scottyctl)\n\
                 Server version: {} (scotty)\n\n\
                 The major or minor versions differ between client and server.\n\
                 Please update {} to ensure compatibility.\n\n\
                 To bypass this check (not recommended), use --bypass-version-check",
                client_version,
                server_version,
                if client_version < server_version {
                    "scottyctl"
                } else {
                    "the scotty server"
                }
            );

            self.ui.eprintln(format!("❌ {}", error_msg).red().to_string());
            return Err(anyhow::anyhow!("Version incompatibility"));
        }

        if client_version.pre != server_version.pre {
            self.ui.eprintln(
                format!(
                    "⚠️  Pre-release versions differ (client: {}, server: {})",
                    if client_version.pre.is_empty() {
                        "stable".to_string()
                    } else {
                        client_version.pre.to_string()
                    },
                    if server_version.pre.is_empty() {
                        "stable".to_string()
                    } else {
                        server_version.pre.to_string()
                    }
                )
                .yellow()
                .to_string(),
            );
        }

        debug!("Version compatibility check passed");
        Ok(())
    }

    async fn get_server_info(&self) -> Result<ServerInfo> {
        // Use the public /api/v1/info endpoint that doesn't require authentication
        let url = format!("{}/api/v1/info", self.server.server);
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .context("Failed to connect to server for version check")?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch server info: HTTP {}",
                response.status()
            ));
        }
        
        let response = response.json::<serde_json::Value>().await
            .context("Failed to parse server info response")?;

        let info: ServerInfo = serde_json::from_value(response)
            .context("Failed to parse server info")?;

        Ok(info)
    }

    fn are_versions_compatible(&self, client: &Version, server: &Version) -> bool {
        client.major == server.major && client.minor == server.minor
    }

    #[allow(dead_code)]
    pub async fn check_auth_mode(&self) -> Result<AuthMode> {
        let server_info = self.get_server_info().await?;
        Ok(server_info.auth_mode)
    }
}