use crate::utils::ui::Ui;
use std::sync::Arc;

/// ServerSettings contain information for connecting to the scotty server
#[derive(Clone)]
pub struct ServerSettings {
    pub server: String,
    pub access_token: Option<String>,
}

/// AppContext provides access to shared application resources
pub struct AppContext {
    /// UI instance for managing terminal output
    pub ui: Arc<Ui>,
    /// Server connection settings
    pub server: ServerSettings,
}

impl AppContext {
    /// Create a new AppContext with the given server settings
    pub fn new(server: ServerSettings) -> Self {
        // Create a single UI instance that will be shared
        let ui = Arc::new(Ui::new());

        AppContext { ui, server }
    }

    /// Get a reference to the UI
    pub fn ui(&self) -> &Arc<Ui> {
        &self.ui
    }

    /// Get a reference to the server settings
    pub fn server(&self) -> &ServerSettings {
        &self.server
    }
}
