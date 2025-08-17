use super::{AuthError, StoredToken};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct TokenStore {
    pub tokens: HashMap<String, StoredToken>,
}

pub struct TokenStorage {
    config_dir: PathBuf,
}

impl TokenStorage {
    pub fn new() -> Result<Self, AuthError> {
        let config_dir = get_config_dir()?;
        Ok(Self { config_dir })
    }

    pub fn save(&self, token: StoredToken) -> Result<(), AuthError> {
        // Ensure config directory exists
        fs::create_dir_all(&self.config_dir)?;

        let server_key = normalize_server_url(&token.server_url);
        let mut token_store = self.load_store()?;
        token_store.tokens.insert(server_key, token);

        let token_file = self.get_token_file();
        let token_json = serde_json::to_string_pretty(&token_store)?;

        fs::write(&token_file, token_json)?;

        // Set secure file permissions (0600 - owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&token_file)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&token_file, perms)?;
        }

        Ok(())
    }

    pub fn load_for_server(&self, server_url: &str) -> Result<Option<StoredToken>, AuthError> {
        let server_key = normalize_server_url(server_url);
        tracing::debug!(
            "Loading token for server: {} (key: {})",
            server_url,
            server_key
        );

        let token_store = self.load_store()?;
        if let Some(token) = token_store.tokens.get(&server_key) {
            tracing::debug!("Found stored token for user: {}", token.user_email);
            Ok(Some(token.clone()))
        } else {
            tracing::debug!("No stored token found for server: {}", server_key);
            Ok(None)
        }
    }

    pub fn clear(&self) -> Result<(), AuthError> {
        let token_file = self.get_token_file();

        if token_file.exists() {
            fs::remove_file(&token_file)?;
        }

        Ok(())
    }

    pub fn clear_for_server(&self, server_url: &str) -> Result<(), AuthError> {
        let server_key = normalize_server_url(server_url);
        let mut token_store = self.load_store()?;

        if token_store.tokens.remove(&server_key).is_some() {
            let token_file = self.get_token_file();
            let token_json = serde_json::to_string_pretty(&token_store)?;
            fs::write(&token_file, token_json)?;
            tracing::debug!("Removed token for server: {}", server_key);
        } else {
            tracing::debug!("No token found for server: {}", server_key);
        }

        Ok(())
    }

    fn load_store(&self) -> Result<TokenStore, AuthError> {
        let token_file = self.get_token_file();
        tracing::debug!("Trying to load token store from: {:?}", token_file);

        if !token_file.exists() {
            tracing::debug!("Token file does not exist, returning empty store");
            return Ok(TokenStore::default());
        }

        let token_json = fs::read_to_string(&token_file)?;
        tracing::debug!("Read token JSON: {}", token_json);

        // Try to parse as new format first, fallback to old format
        if let Ok(token_store) = serde_json::from_str::<TokenStore>(&token_json) {
            tracing::debug!(
                "Parsed as new token store format with {} tokens",
                token_store.tokens.len()
            );
            Ok(token_store)
        } else if let Ok(old_token) = serde_json::from_str::<StoredToken>(&token_json) {
            // Migrate from old single-token format
            tracing::debug!("Found old format token, migrating to new format");
            let server_key = normalize_server_url(&old_token.server_url);
            let mut tokens = HashMap::new();
            tokens.insert(server_key, old_token);
            Ok(TokenStore { tokens })
        } else {
            tracing::error!("Failed to parse token file");
            Err(AuthError::Json(
                serde_json::from_str::<TokenStore>(&token_json).unwrap_err(),
            ))
        }
    }

    fn get_token_file(&self) -> PathBuf {
        self.config_dir.join("tokens.json")
    }
}

fn get_config_dir() -> Result<PathBuf, AuthError> {
    // Force use of ~/.config/scottyctl instead of platform-specific directories
    let home_dir = std::env::var("HOME").map_err(|_| AuthError::ConfigDirNotFound)?;
    Ok(PathBuf::from(home_dir).join(".config").join("scottyctl"))
}

fn normalize_server_url(server_url: &str) -> String {
    // Remove trailing slashes and normalize the URL for consistent storage keys
    let normalized = server_url.trim_end_matches('/').to_string();

    // Add default port if none specified
    if normalized.starts_with("http://")
        && !normalized.contains(":80")
        && normalized.matches(':').count() == 1
    {
        // HTTP without explicit port - no modification needed
        // normalized is already correct
    } else if normalized.starts_with("https://")
        && !normalized.contains(":443")
        && normalized.matches(':').count() == 1
    {
        // HTTPS without explicit port - no modification needed
        // normalized is already correct
    }

    normalized
}
