use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
#[readonly::make]
pub struct ApiServer {
    pub bind_address: String,
}
#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
#[readonly::make]
pub struct Settings {
    pub debug: bool,
    pub telemetry: Option<String>,
    pub api: ApiServer,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("YAFBDS_RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = Config::builder()
            .set_default("api.bind_address", "0.0.0.0:8080")?
            // Start off by merging in the "default" configuration file
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            .add_source(File::with_name("config/local").required(false))
            .add_source(
                Environment::default()
                    .prefix("YAFBDS")
                    .prefix_separator("_")
                    .separator("_")
                    .try_parsing(true),
            )
            .build()?;

        let mut settings: Settings = s.try_deserialize()?;

        // Check if we should serve the frontend.
        // We check for special strings here, so we can override and disable
        // config via environment variables, even if set in the default config
        settings.telemetry = settings.check_if_optional(&settings.telemetry);
        Ok(settings)
    }

    fn check_if_optional(&self, s: &Option<String>) -> Option<String> {
        match s {
            None => None,
            Some(s) => match s.to_lowercase().as_str() {
                "no" | "false" | "0" => None,
                _ => Some(s.to_string()),
            },
        }
    }
}
