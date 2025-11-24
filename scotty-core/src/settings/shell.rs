use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for shell sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellSettings {
    /// Time-to-live for shell sessions in seconds
    pub session_ttl_seconds: u64,
    /// Maximum number of concurrent shell sessions per app
    pub max_sessions_per_app: usize,
    /// Maximum number of concurrent shell sessions globally
    pub max_sessions_global: usize,
    /// Default shell command to execute
    pub default_shell: String,
    /// Environment variables to set in shell sessions
    pub default_env: std::collections::HashMap<String, String>,
}

impl Default for ShellSettings {
    fn default() -> Self {
        Self {
            session_ttl_seconds: 3600, // 1 hour
            max_sessions_per_app: 5,
            max_sessions_global: 50,
            default_shell: "/bin/sh".to_string(),
            default_env: std::collections::HashMap::new(),
        }
    }
}

impl ShellSettings {
    /// Get the session TTL as a Duration
    pub fn session_ttl(&self) -> Duration {
        Duration::from_secs(self.session_ttl_seconds)
    }
}
