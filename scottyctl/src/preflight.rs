use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use crate::context::ServerSettings;
use crate::utils::ui::Ui;
use owo_colors::OwoColorize;
use scotty_core::api::ServerInfo;
use scotty_core::http::HttpClient;
use scotty_core::settings::api_server::AuthMode;
use scotty_core::version::VersionManager;
use std::sync::Arc;
use std::time::Duration;

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

        let client_version =
            VersionManager::current_version().context("Failed to parse client version")?;

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

        let server_version = VersionManager::parse_version(&server_info.version)
            .context("Failed to parse server version")?;

        debug!(
            "Version check - Client: {}, Server: {}",
            client_version, server_version
        );

        if !VersionManager::are_compatible(&client_version, &server_version) {
            let update_recommendation =
                VersionManager::get_update_recommendation(&client_version, &server_version)
                    .expect("Should have update recommendation for incompatible versions");

            let error_msg = format!(
                "Version incompatibility detected!\n\
                 {}\n\n\
                 The major or minor versions differ between client and server.\n\
                 Please update {} to ensure compatibility.\n\n\
                 To bypass this check (not recommended), use --bypass-version-check",
                VersionManager::format_version_comparison(&client_version, &server_version),
                update_recommendation
            );

            self.ui
                .eprintln(format!("❌ {}", error_msg).red().to_string());
            return Err(anyhow::anyhow!("Version incompatibility"));
        }

        if client_version.pre != server_version.pre {
            self.ui.eprintln(
                format!(
                    "⚠️  Pre-release versions differ (client: {}, server: {})",
                    VersionManager::prerelease_type(&client_version),
                    VersionManager::prerelease_type(&server_version)
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
        let client = HttpClient::with_timeout(Duration::from_secs(5))
            .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

        match client.get_json::<ServerInfo>(&url).await {
            Ok(info) => Ok(info),
            Err(e) => Err(anyhow::anyhow!(
                "Failed to connect to server for version check: {}",
                e
            )),
        }
    }

    #[allow(dead_code)]
    pub async fn check_auth_mode(&self) -> Result<AuthMode> {
        let server_info = self.get_server_info().await?;
        Ok(server_info.auth_mode)
    }
}
