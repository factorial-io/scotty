use super::{AuthError, OAuthConfig, StoredToken};
use scotty_core::auth::{DeviceFlowResponse, ErrorResponse, TokenResponse};
use scotty_core::http::HttpClient;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;

const DEVICE_CODE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";
const CLIENT_ID: &str = "scottyctl";

pub struct DeviceFlowClient {
    client: HttpClient,
    // Retained for the OAuth config validation it carries (client_id, issuer);
    // the device endpoints use `user_provided_server_url`, not this.
    #[allow(dead_code)]
    config: OAuthConfig,
    user_provided_server_url: String,
}

impl DeviceFlowClient {
    pub fn new(config: OAuthConfig, user_provided_server_url: String) -> Result<Self, AuthError> {
        let client = HttpClient::with_timeout(Duration::from_secs(30))
            .map_err(|_| AuthError::ServerError)?;

        Ok(Self {
            client,
            config,
            user_provided_server_url,
        })
    }

    pub async fn start_device_flow(&self) -> Result<DeviceFlowResponse, AuthError> {
        let device_url = format!("{}/oauth/device", self.user_provided_server_url);
        tracing::info!("Starting device flow at {}", device_url);

        self.client
            .post_json::<serde_json::Value, DeviceFlowResponse>(&device_url, &serde_json::json!({}))
            .await
            .map_err(|_| AuthError::ServerError)
    }

    /// Poll until the login is approved, denied or times out (RFC 8628 §3.5).
    pub async fn poll_for_token(
        &self,
        device_code: &str,
        interval_seconds: u64,
        timeout_seconds: u64,
    ) -> Result<StoredToken, AuthError> {
        let start_time = SystemTime::now();
        let timeout_duration = Duration::from_secs(timeout_seconds);
        let mut interval = Duration::from_secs(interval_seconds.max(1));

        loop {
            if start_time.elapsed().unwrap_or_default() > timeout_duration {
                return Err(AuthError::Timeout);
            }
            sleep(interval).await;

            let form = [
                ("grant_type", DEVICE_CODE_GRANT),
                ("device_code", device_code),
            ];
            match token_request(&self.user_provided_server_url, &form).await {
                Ok(token) => return Ok(token),
                Err(AuthError::AuthorizationPending) => continue,
                Err(AuthError::SlowDown) => interval += Duration::from_secs(5),
                Err(e) => return Err(e),
            }
        }
    }
}

/// Refresh the CLI session: a new access and refresh token pair (the used
/// refresh token is spent).
pub async fn refresh_session(
    server_url: &str,
    token: &StoredToken,
) -> Result<StoredToken, AuthError> {
    let refresh_token = token
        .refresh_token
        .as_deref()
        .ok_or(AuthError::SessionExpired)?;
    let form = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", CLIENT_ID),
    ];
    match token_request(server_url, &form).await {
        Err(AuthError::InvalidGrant) => Err(AuthError::SessionExpired),
        other => other,
    }
}

/// One request to `{server_url}/oauth/device/token`.
async fn token_request(server_url: &str, form: &[(&str, &str)]) -> Result<StoredToken, AuthError> {
    let url = format!("{}/oauth/device/token", server_url.trim_end_matches('/'));
    let client =
        HttpClient::with_timeout(Duration::from_secs(30)).map_err(|_| AuthError::ServerError)?;
    let response = client.inner().post(&url).form(form).send().await?;
    let status = response.status();

    if status.is_success() {
        let token: TokenResponse = response.json().await?;
        return Ok(StoredToken {
            access_token: token.access_token,
            refresh_token: token.refresh_token,
            expires_at: token
                .expires_in
                .map(|s| SystemTime::now() + Duration::from_secs(s)),
            user_email: token.user_email,
            user_name: token.user_name,
            server_url: server_url.to_string(),
        });
    }

    let error = response
        .json::<ErrorResponse>()
        .await
        .map_err(|_| AuthError::ServerError)?;
    tracing::debug!(
        "Token endpoint refused ({}): {} - {}",
        status,
        error.error,
        error.description()
    );
    Err(match error.error.as_str() {
        "authorization_pending" => AuthError::AuthorizationPending,
        "slow_down" => AuthError::SlowDown,
        "access_denied" => AuthError::AccessDenied,
        "expired_token" => AuthError::Timeout,
        "invalid_grant" => AuthError::InvalidGrant,
        _ => AuthError::ServerError,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn stored(server_url: &str, refresh_token: Option<&str>) -> StoredToken {
        StoredToken {
            access_token: "old-access".into(),
            refresh_token: refresh_token.map(str::to_string),
            expires_at: None,
            user_email: "alice@example.com".into(),
            user_name: "alice@example.com".into(),
            server_url: server_url.into(),
        }
    }

    #[tokio::test]
    async fn refresh_sends_the_refresh_grant_and_keeps_the_new_pair() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/device/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("refresh_token=r1"))
            .and(body_string_contains("client_id=scottyctl"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "a2",
                "token_type": "Bearer",
                "user_id": "alice@example.com",
                "user_name": "alice@example.com",
                "user_email": "alice@example.com",
                "user_picture": null,
                "refresh_token": "r2",
                "expires_in": 43200
            })))
            .expect(1)
            .mount(&server)
            .await;

        let token = refresh_session(&server.uri(), &stored(&server.uri(), Some("r1")))
            .await
            .unwrap();
        assert_eq!(token.access_token, "a2");
        assert_eq!(token.refresh_token.as_deref(), Some("r2"));
        assert!(token.expires_at.is_some());
    }

    #[tokio::test]
    async fn refused_refresh_means_log_in_again() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/device/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "invalid_grant",
                "error_description": "revoked"
            })))
            .mount(&server)
            .await;

        let result = refresh_session(&server.uri(), &stored(&server.uri(), Some("r1"))).await;
        assert!(matches!(result, Err(AuthError::SessionExpired)));
        let result = refresh_session(&server.uri(), &stored(&server.uri(), None)).await;
        assert!(matches!(result, Err(AuthError::SessionExpired)));
    }
}
