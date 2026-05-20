use serde::Deserialize;

/// Default maximum transfer size for file uploads/downloads (1 GiB).
pub const DEFAULT_MAX_TRANSFER_SIZE: u64 = 1024 * 1024 * 1024;

/// File transfer related settings.
#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct FilesSettings {
    /// Maximum size in bytes allowed for a single file transfer (upload or
    /// download). Both directions are aborted with `413 Payload Too Large`
    /// when this threshold is exceeded.
    ///
    /// Configurable via `SCOTTY__FILES__MAX_TRANSFER_SIZE`.
    #[serde(default = "default_max_transfer_size")]
    pub max_transfer_size: u64,
}

fn default_max_transfer_size() -> u64 {
    DEFAULT_MAX_TRANSFER_SIZE
}

impl Default for FilesSettings {
    fn default() -> Self {
        Self {
            max_transfer_size: DEFAULT_MAX_TRANSFER_SIZE,
        }
    }
}
