use serde::{Deserialize, Serialize};

/// Configuration for output collection and limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSettings {
    /// Maximum number of lines to keep in memory per task
    pub max_lines: usize,
    /// Maximum length of a single line (characters)
    pub max_line_length: usize,
    /// Maximum number of lines to return in API responses by default
    pub api_default_limit: usize,
    /// Maximum number of lines that can be requested via API
    pub api_max_limit: usize,
}

impl Default for OutputSettings {
    fn default() -> Self {
        Self {
            max_lines: 10000,
            max_line_length: 4096,
            api_default_limit: 100,
            api_max_limit: 1000,
        }
    }
}
