use super::{AuthError, StoredToken};
use crate::auth::storage::TokenStorage;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Cached token manager that loads tokens once and keeps them in memory
#[derive(Debug, Clone)]
pub struct CachedTokenManager {
    inner: Arc<RwLock<CachedTokenManagerInner>>,
}

#[derive(Debug)]
struct CachedTokenManagerInner {
    /// Cache of tokens per server URL
    token_cache: HashMap<String, Option<StoredToken>>,
    /// Underlying storage for persistent operations
    storage: TokenStorage,
}

impl CachedTokenManager {
    /// Create a new cached token manager
    pub fn new() -> Result<Self, AuthError> {
        let storage = TokenStorage::new()?;
        Ok(Self {
            inner: Arc::new(RwLock::new(CachedTokenManagerInner {
                token_cache: HashMap::new(),
                storage,
            })),
        })
    }

    /// Load token for a server, using cache if available
    pub fn load_for_server(&self, server_url: &str) -> Result<Option<StoredToken>, AuthError> {
        let server_key = normalize_server_url(server_url);

        // First check cache
        {
            let inner = self.inner.read().unwrap();
            if let Some(cached_token) = inner.token_cache.get(&server_key) {
                tracing::debug!("Token cache hit for server: {}", server_key);
                return Ok(cached_token.clone());
            }
        }

        // Cache miss - load from storage and cache result
        tracing::debug!(
            "Token cache miss for server: {}, loading from storage",
            server_key
        );
        let token = {
            let inner = self.inner.read().unwrap();
            inner.storage.load_for_server(server_url)?
        };

        // Cache the result (even if None)
        {
            let mut inner = self.inner.write().unwrap();
            inner.token_cache.insert(server_key.clone(), token.clone());
        }

        tracing::debug!("Cached token for server: {}", server_key);
        Ok(token)
    }

    /// Save a token and update cache
    pub fn save(&self, token: StoredToken) -> Result<(), AuthError> {
        let server_key = normalize_server_url(&token.server_url);

        // Save to persistent storage
        {
            let inner = self.inner.read().unwrap();
            inner.storage.save(token.clone())?;
        }

        // Update cache
        {
            let mut inner = self.inner.write().unwrap();
            inner.token_cache.insert(server_key.clone(), Some(token));
        }

        tracing::debug!("Saved and cached token for server: {}", server_key);
        Ok(())
    }

    /// Clear token for a server and update cache
    pub fn clear_for_server(&self, server_url: &str) -> Result<(), AuthError> {
        let server_key = normalize_server_url(server_url);

        // Clear from persistent storage
        {
            let inner = self.inner.read().unwrap();
            inner.storage.clear_for_server(server_url)?;
        }

        // Update cache
        {
            let mut inner = self.inner.write().unwrap();
            inner.token_cache.insert(server_key.clone(), None);
        }

        tracing::debug!("Cleared and uncached token for server: {}", server_key);
        Ok(())
    }

    /// Clear all cached tokens (forces reload from storage on next access)
    pub fn clear_cache(&self) {
        let mut inner = self.inner.write().unwrap();
        inner.token_cache.clear();
        tracing::debug!("Cleared token cache");
    }

    /// Get number of cached tokens (for debugging)
    pub fn cache_size(&self) -> usize {
        let inner = self.inner.read().unwrap();
        inner.token_cache.len()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_server_url() {
        assert_eq!(
            normalize_server_url("http://localhost:3000"),
            "http://localhost:3000"
        );
        assert_eq!(
            normalize_server_url("http://localhost:3000/"),
            "http://localhost:3000"
        );
        assert_eq!(
            normalize_server_url("https://api.example.com"),
            "https://api.example.com"
        );
        assert_eq!(
            normalize_server_url("https://api.example.com/"),
            "https://api.example.com"
        );
    }
}
