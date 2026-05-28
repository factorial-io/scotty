use serde::Deserialize;

/// Default maximum transfer size for file uploads/downloads (1 GiB).
///
/// Re-exported from `scotty-types` so the value has a single source of truth
/// shared between the settings layer and the wire types.
pub use scotty_types::files::DEFAULT_MAX_TRANSFER_SIZE;

/// File transfer related settings.
#[derive(Debug, Deserialize, Clone)]
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
