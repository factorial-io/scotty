use super::{DeviceFlowStore, OAuthSessionStore, WebFlowStore};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tracing::{debug, info};

#[cfg(test)]
use scotty_core::utils::secret::MaskedSecret;

/// Trait for sessions that can expire
pub trait ExpirableSession {
    fn expires_at(&self) -> SystemTime;
}

impl ExpirableSession for super::DeviceFlowSession {
    fn expires_at(&self) -> SystemTime {
        self.expires_at
    }
}

impl ExpirableSession for super::WebFlowSession {
    fn expires_at(&self) -> SystemTime {
        self.expires_at
    }
}

impl ExpirableSession for super::OAuthSession {
    fn expires_at(&self) -> SystemTime {
        self.expires_at
    }
}

/// Generic cleanup function for any session store
fn cleanup_sessions<S: ExpirableSession>(
    store: Arc<Mutex<HashMap<String, S>>>,
    session_type: &str,
) -> usize {
    let mut sessions = store.lock().unwrap();
    let initial_count = sessions.len();
    let now = SystemTime::now();

    sessions.retain(|session_id, session| {
        let is_valid = session.expires_at() > now;
        if !is_valid {
            debug!(
                "Removing expired {} session: session_id={}",
                session_type, session_id
            );
        }
        is_valid
    });

    let removed_count = initial_count - sessions.len();
    if removed_count > 0 {
        info!(
            "Cleaned up {} expired {} session(s), {} remaining",
            removed_count,
            session_type,
            sessions.len()
        );
        // Record metrics
        if let Some(metrics) = crate::metrics::get_metrics() {
            metrics
                .oauth_sessions_expired_cleaned
                .add(removed_count as u64, &[]);
        }
    }
    removed_count
}

/// Clean up expired sessions from the device flow store
pub fn cleanup_device_flow_sessions(store: DeviceFlowStore) -> usize {
    cleanup_sessions(store, "device flow")
}

/// Clean up expired sessions from the web flow store
pub fn cleanup_web_flow_sessions(store: WebFlowStore) -> usize {
    cleanup_sessions(store, "web flow")
}

/// Clean up expired sessions from the OAuth session store
pub fn cleanup_oauth_sessions(store: OAuthSessionStore) -> usize {
    cleanup_sessions(store, "OAuth")
}

/// Sample OAuth session counts for metrics
pub fn sample_oauth_session_metrics(
    device_flow_store: DeviceFlowStore,
    web_flow_store: WebFlowStore,
    session_store: OAuthSessionStore,
) {
    if let Some(metrics) = crate::metrics::get_metrics() {
        let device_count = device_flow_store.lock().unwrap().len() as i64;
        let web_count = web_flow_store.lock().unwrap().len() as i64;
        let session_count = session_store.lock().unwrap().len() as i64;

        metrics
            .oauth_device_flow_sessions_active
            .record(device_count, &[]);
        metrics
            .oauth_web_flow_sessions_active
            .record(web_count, &[]);
        metrics.oauth_sessions_active.record(session_count, &[]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oauth::{DeviceFlowSession, OAuthSession, WebFlowSession};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[test]
    fn test_cleanup_device_flow_sessions() {
        let store = Arc::new(Mutex::new(HashMap::new()));

        // Add expired session
        {
            let mut sessions = store.lock().unwrap();
            sessions.insert(
                "expired_device_code".to_string(),
                DeviceFlowSession {
                    device_code: "expired_device_code".to_string(),
                    user_code: "EXPIRED".to_string(),
                    verification_uri: "https://example.com/device".to_string(),
                    expires_at: SystemTime::now() - Duration::from_secs(60),
                    oidc_access_token: None,
                    completed: false,
                    interval: 5,
                },
            );
        }

        // Add valid session
        {
            let mut sessions = store.lock().unwrap();
            sessions.insert(
                "valid_device_code".to_string(),
                DeviceFlowSession {
                    device_code: "valid_device_code".to_string(),
                    user_code: "VALID".to_string(),
                    verification_uri: "https://example.com/device".to_string(),
                    expires_at: SystemTime::now() + Duration::from_secs(600),
                    oidc_access_token: None,
                    completed: false,
                    interval: 5,
                },
            );
        }

        let removed = cleanup_device_flow_sessions(store.clone());
        assert_eq!(removed, 1);

        let sessions = store.lock().unwrap();
        assert_eq!(sessions.len(), 1);
        assert!(sessions.contains_key("valid_device_code"));
    }

    #[test]
    fn test_cleanup_web_flow_sessions() {
        let store = Arc::new(Mutex::new(HashMap::new()));

        // Add expired session
        {
            let mut sessions = store.lock().unwrap();
            sessions.insert(
                "expired_session_id".to_string(),
                WebFlowSession {
                    csrf_token: MaskedSecret::new("expired_csrf".to_string()),
                    pkce_verifier: MaskedSecret::new("expired_verifier".to_string()),
                    redirect_url: "https://example.com/redirect".to_string(),
                    frontend_callback_url: None,
                    expires_at: SystemTime::now() - Duration::from_secs(60),
                },
            );
        }

        // Add valid session
        {
            let mut sessions = store.lock().unwrap();
            sessions.insert(
                "valid_session_id".to_string(),
                WebFlowSession {
                    csrf_token: MaskedSecret::new("valid_csrf".to_string()),
                    pkce_verifier: MaskedSecret::new("valid_verifier".to_string()),
                    redirect_url: "https://example.com/redirect".to_string(),
                    frontend_callback_url: None,
                    expires_at: SystemTime::now() + Duration::from_secs(600),
                },
            );
        }

        let removed = cleanup_web_flow_sessions(store.clone());
        assert_eq!(removed, 1);

        let sessions = store.lock().unwrap();
        assert_eq!(sessions.len(), 1);
        assert!(sessions.contains_key("valid_session_id"));
    }

    #[test]
    fn test_cleanup_oauth_sessions() {
        let store = Arc::new(Mutex::new(HashMap::new()));

        // Add expired session
        {
            let mut sessions = store.lock().unwrap();
            sessions.insert(
                "expired_oauth_session".to_string(),
                OAuthSession {
                    oidc_token: "expired_token".to_string(),
                    user: crate::oauth::device_flow::OidcUser {
                        id: "user123".to_string(),
                        username: Some("testuser".to_string()),
                        name: Some("Test User".to_string()),
                        given_name: None,
                        family_name: None,
                        nickname: None,
                        picture: None,
                        website: None,
                        locale: None,
                        zoneinfo: None,
                        updated_at: None,
                        email: Some("test@example.com".to_string()),
                        email_verified: Some(true),
                        custom_claims: HashMap::new(),
                    },
                    expires_at: SystemTime::now() - Duration::from_secs(60),
                },
            );
        }

        // Add valid session
        {
            let mut sessions = store.lock().unwrap();
            sessions.insert(
                "valid_oauth_session".to_string(),
                OAuthSession {
                    oidc_token: "valid_token".to_string(),
                    user: crate::oauth::device_flow::OidcUser {
                        id: "user456".to_string(),
                        username: Some("validuser".to_string()),
                        name: Some("Valid User".to_string()),
                        given_name: None,
                        family_name: None,
                        nickname: None,
                        picture: None,
                        website: None,
                        locale: None,
                        zoneinfo: None,
                        updated_at: None,
                        email: Some("valid@example.com".to_string()),
                        email_verified: Some(true),
                        custom_claims: HashMap::new(),
                    },
                    expires_at: SystemTime::now() + Duration::from_secs(600),
                },
            );
        }

        let removed = cleanup_oauth_sessions(store.clone());
        assert_eq!(removed, 1);

        let sessions = store.lock().unwrap();
        assert_eq!(sessions.len(), 1);
        assert!(sessions.contains_key("valid_oauth_session"));
    }
}
