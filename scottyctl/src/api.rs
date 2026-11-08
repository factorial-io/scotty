use anyhow::Context;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde_json::Value;
use tracing::info;

use crate::auth::cache::CachedTokenManager;
use crate::auth::config::get_server_info;
use crate::auth::device_flow::refresh_session;
use crate::context::ServerSettings;
use crate::utils::ui::Ui;
use owo_colors::OwoColorize;
use scotty_core::http::{HttpClient, HttpError, RetryError};
use scotty_core::settings::api_server::AuthMode;
use scotty_core::tasks::running_app_context::RunningAppContext;
use scotty_core::tasks::task_details::State;
use scotty_core::version::VersionManager;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

// Global cached token manager
static CACHED_TOKEN_MANAGER: OnceLock<CachedTokenManager> = OnceLock::new();

fn get_cached_token_manager() -> &'static CachedTokenManager {
    CACHED_TOKEN_MANAGER.get_or_init(|| {
        CachedTokenManager::new().expect("Failed to initialize cached token manager")
    })
}

fn create_authenticated_client(token: &str) -> anyhow::Result<HttpClient> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token))
            .context("Failed to create authorization header")?,
    );

    // Add user agent with version
    let version = VersionManager::current_version()
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(&format!("scottyctl/{}", version))
            .context("Failed to create user agent header")?,
    );

    HttpClient::builder()
        .with_timeout(Duration::from_secs(10))
        .with_default_headers(headers)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))
}

/// Helper function to normalize URLs by handling trailing slashes
fn normalize_url(base_url: &str, path: &str) -> String {
    let mut normalized_base = base_url.trim_end_matches('/').to_string();
    let normalized_path = path.trim_start_matches('/');

    normalized_base.push('/');
    normalized_base.push_str(normalized_path);
    normalized_base
}

pub async fn get_auth_token(server: &ServerSettings) -> Result<String, anyhow::Error> {
    // 1. First check for explicit access token (highest priority)
    // This allows users to override cached OAuth tokens with --access-token or SCOTTY_ACCESS_TOKEN
    if let Some(token) = &server.access_token {
        return Ok(token.clone());
    }

    // 2. Check server auth mode to determine if OAuth tokens should be used
    let server_supports_oauth = match get_server_info(server).await {
        Ok(server_info) => server_info.auth_mode == AuthMode::OAuth,
        Err(_) => false, // If we can't check, assume OAuth is not supported
    };

    // 3. Try stored OAuth token only if server supports OAuth. An expired
    // one is refreshed on the 401 it gets (`send_with_refresh`).
    if server_supports_oauth {
        if let Ok(Some(stored_token)) = get_cached_token_manager().load_for_server(&server.server) {
            return Ok(stored_token.access_token);
        }
    }

    Err(anyhow::anyhow!(
        "No authentication available. Run 'scottyctl auth:login' or set SCOTTY_ACCESS_TOKEN"
    ))
}

/// Send `call` with `token`. On a 401, ask `refresh` once for a new token
/// and send again; without one, the 401 stands.
async fn send_with_refresh<T, C, CF, R, RF>(
    token: String,
    call: C,
    refresh: R,
) -> Result<T, RetryError>
where
    C: Fn(String) -> CF,
    CF: std::future::Future<Output = Result<T, RetryError>>,
    R: FnOnce() -> RF,
    RF: std::future::Future<Output = Option<String>>,
{
    match call(token).await {
        Err(e) if e.status_code() == Some(401) => match refresh().await {
            Some(token) => call(token).await,
            None => Err(e),
        },
        result => result,
    }
}

/// Refresh the stored OAuth session of `server` and persist the new pair;
/// the new access token, or `None` (explicit tokens are never refreshed).
async fn refresh_stored_session(server: &ServerSettings) -> Option<String> {
    if server.access_token.is_some() {
        return None;
    }
    let manager = get_cached_token_manager();
    let stored = manager.load_for_server(&server.server).ok()??;
    match refresh_session(&server.server, &stored).await {
        Ok(token) => {
            let access_token = token.access_token.clone();
            if let Err(e) = manager.save(token) {
                tracing::warn!("Failed to save the refreshed session: {}", e);
            }
            Some(access_token)
        }
        Err(e) => {
            tracing::debug!("Session refresh failed: {}", e);
            None
        }
    }
}

async fn send_request(
    token: &str,
    url: &str,
    method: &str,
    body: Option<&Value>,
) -> Result<Value, RetryError> {
    // HttpClient already has retry logic built-in, no need to wrap it
    let client = create_authenticated_client(token)
        .map_err(|e| RetryError::NonRetriable(HttpError::ParseError(e.to_string())))?;

    match method.to_lowercase().as_str() {
        "post" => {
            let empty = serde_json::json!({});
            client
                .post_json::<Value, Value>(url, body.unwrap_or(&empty))
                .await
        }
        "delete" => {
            // DELETE requests with JSON bodies need manual handling since there's no delete_json method
            let response = match body {
                Some(body) => {
                    client
                        .request_with_body(reqwest::Method::DELETE, url, body)
                        .await?
                }
                None => client.request(reqwest::Method::DELETE, url).await?,
            };
            response
                .json::<Value>()
                .await
                .map_err(|e| RetryError::NonRetriable(HttpError::ParseError(e.to_string())))
        }
        _ => client.get_json::<Value>(url).await,
    }
}

pub async fn get_or_post(
    server: &ServerSettings,
    action: &str,
    method: &str,
    body: Option<Value>,
) -> anyhow::Result<Value> {
    let token = get_auth_token(server).await?;
    let url = normalize_url(&server.server, &format!("api/v1/authenticated/{}", action));
    info!("Calling scotty API at {}", &url);

    let result = send_with_refresh(
        token,
        |token| {
            let (url, body) = (&url, body.as_ref());
            async move { send_request(&token, url, method, body).await }
        },
        || refresh_stored_session(server),
    )
    .await;

    match result {
        Ok(value) => Ok(value),
        Err(RetryError::NonRetriable(err)) if err.status_code() == Some(401) => {
            Err(anyhow::anyhow!(
                "Not authenticated or session expired. Run 'scottyctl --server {} auth:login' or set SCOTTY_ACCESS_TOKEN",
                server.server
            ))
        }
        Err(RetryError::NonRetriable(err)) => {
            // HttpError now preserves the status code
            if err.is_client_error() {
                Err(anyhow::anyhow!(
                    "Client error calling scotty API at {}: {}",
                    url,
                    err
                ))
            } else {
                Err(anyhow::anyhow!(
                    "Failed to call scotty API at {}: {}",
                    url,
                    err
                ))
            }
        }
        Err(RetryError::ExhaustedRetries { error, attempts }) => Err(anyhow::anyhow!(
            "Failed to call scotty API at {} after {} retries: {}",
            url,
            attempts,
            error
        )),
    }
}

/// Revoke the stored OAuth session of `server` on the server
/// (`POST /api/v1/authenticated/logout`), refreshing once on a 401.
/// `Ok(false)` when there is no stored session; an explicit access token is
/// never revoked.
pub async fn revoke_stored_session(server: &ServerSettings) -> anyhow::Result<bool> {
    if server.access_token.is_some() {
        return Ok(false);
    }
    let Some(stored) = get_cached_token_manager().load_for_server(&server.server)? else {
        return Ok(false);
    };
    let url = normalize_url(&server.server, "api/v1/authenticated/logout");
    send_with_refresh(
        stored.access_token,
        |token| post_logout(&url, token),
        || refresh_stored_session(server),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to revoke the session at {}: {}", server.server, e))?;
    Ok(true)
}

/// `POST url` with `token`; any 2xx (the server answers 204) is success.
async fn post_logout(url: &str, token: String) -> Result<(), RetryError> {
    let response = reqwest::Client::new()
        .post(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| RetryError::NonRetriable(HttpError::Network(e)))?;
    let status = response.status();
    if status.is_success() {
        Ok(())
    } else {
        Err(RetryError::NonRetriable(HttpError::http(
            status.as_u16(),
            status.canonical_reason().unwrap_or("error"),
        )))
    }
}

pub async fn get(server: &ServerSettings, method: &str) -> anyhow::Result<Value> {
    get_or_post(server, method, "GET", None).await
}

pub async fn post(server: &ServerSettings, method: &str, body: Value) -> anyhow::Result<Value> {
    get_or_post(server, method, "post", Some(body)).await
}

pub async fn delete(
    server: &ServerSettings,
    method: &str,
    body: Option<Value>,
) -> anyhow::Result<Value> {
    get_or_post(server, method, "delete", body).await
}

/// Get with access to HttpError for better error handling
/// This bypasses the anyhow error conversion in get_or_post to preserve HTTP status codes
pub async fn get_with_error(server: &ServerSettings, method: &str) -> Result<Value, HttpError> {
    let token = get_auth_token(server)
        .await
        .map_err(|e| HttpError::ParseError(format!("Failed to get auth token: {}", e)))?;

    let url = normalize_url(&server.server, &format!("api/v1/authenticated/{}", method));
    info!("Calling scotty API at {}", &url);

    let result = send_with_refresh(
        token,
        |token| {
            let url = &url;
            async move { send_request(&token, url, "get", None).await }
        },
        || refresh_stored_session(server),
    )
    .await;

    match result {
        Ok(value) => Ok(value),
        Err(RetryError::NonRetriable(http_err)) => Err(http_err),
        Err(RetryError::ExhaustedRetries { error, .. }) => Err(error),
    }
}

pub async fn wait_for_task(
    server: &ServerSettings,
    context: &RunningAppContext,
    ui: &Arc<Ui>,
    ws_connection: Result<crate::websocket::AuthenticatedWebSocket, anyhow::Error>,
) -> anyhow::Result<()> {
    use crate::utils::status_line::Status;
    use futures_util::{SinkExt, StreamExt};
    use scotty_core::tasks::task_details::TaskDetails;
    use scotty_core::websocket::message::WebSocketMessage;
    use scotty_types::TaskOutputData;
    use tokio_tungstenite::tungstenite::Message;

    // If WebSocket connected successfully, use it for real-time streaming
    if let Ok(mut ws) = ws_connection {
        tracing::debug!("Using WebSocket for real-time task output streaming");
        let start_message = WebSocketMessage::StartTaskOutputStream {
            task_id: context.task.id,
            from_beginning: true, // Get all output from the beginning
        };

        // Send the request but don't fail if it doesn't work
        let _ = ws.send(start_message).await;

        // Split WebSocket into sender and receiver
        let (mut ws_sender, mut ws_receiver) = ws.split();

        // Shared flag to signal when WebSocket stream has ended
        let stream_ended = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stream_ended_clone = stream_ended.clone();

        // Spawn a task to handle WebSocket messages
        let ui_clone = ui.clone();
        let ws_handle = tokio::spawn(async move {
            while let Some(message) = ws_receiver.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                            match ws_message {
                                WebSocketMessage::TaskOutputData(TaskOutputData {
                                    lines, ..
                                }) => {
                                    for line in lines {
                                        display_task_output_line(&line, &ui_clone);
                                    }
                                }
                                WebSocketMessage::TaskOutputStreamEnded { .. } => {
                                    stream_ended_clone
                                        .store(true, std::sync::atomic::Ordering::SeqCst);
                                    break; // Stop listening when stream ends
                                }
                                _ => {} // Ignore other message types
                            }
                        }
                    }
                    Ok(Message::Close(_)) | Err(_) => break,
                    _ => {}
                }
            }
            // ws_receiver is dropped here which is fine
        });

        // Poll for task completion status
        let mut done = false;
        while !done {
            let result = get(server, &format!("task/{}", context.task.id)).await?;
            let task: TaskDetails =
                serde_json::from_value(result).context("Failed to parse task")?;

            // Check if task is done
            done = task.state != State::Running;

            if done {
                // Wait a bit for any remaining WebSocket messages to be processed
                // The server should send TaskOutputStreamEnded when the task finishes
                let mut wait_attempts = 0;
                while !stream_ended.load(std::sync::atomic::Ordering::SeqCst) && wait_attempts < 20
                {
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    wait_attempts += 1;
                }

                // Now abort the WebSocket handler
                ws_handle.abort();

                match task.state {
                    State::Finished => {
                        if task.last_exit_code.is_none() || task.last_exit_code == Some(0) {
                            ui.set_status("Task completed successfully", Status::Succeeded);
                        } else {
                            ui.set_status("Task failed", Status::Failed);
                            if let Some(exit_code) = task.last_exit_code {
                                ui.eprintln(format!("Exit code: {}", exit_code).red().to_string());
                            }
                        }
                    }
                    State::Failed => {
                        ui.set_status("Task failed", Status::Failed);
                        // A task can fail without a failing subprocess; do not
                        // show a misleading "Exit code: 0".
                        if let Some(exit_code) = task.last_exit_code.filter(|c| *c != 0) {
                            ui.eprintln(format!("Exit code: {}", exit_code).red().to_string());
                        }
                    }
                    State::Running => {} // Should not happen since we check above
                }

                // Return error if task failed
                // The server may fail a task without a failing subprocess (e.g. a
                // panic or a failed app-data refresh), so the state is authoritative.
                if task.state == State::Failed || task.last_exit_code.is_some_and(|c| c != 0) {
                    return Err(match task.last_exit_code {
                        Some(c) if c != 0 => anyhow::anyhow!("Task failed with exit code {}", c),
                        _ => anyhow::anyhow!("Task failed"),
                    });
                }
            } else {
                // Poll every 500ms
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }

        // Clean up WebSocket connection
        let _ = ws_sender.close().await;
    } else {
        // Fallback to simple polling without WebSocket output
        if let Err(ref e) = ws_connection {
            tracing::warn!(
                "WebSocket connection failed, falling back to polling mode (real-time output unavailable): {}",
                e
            );
        }

        let mut done = false;
        while !done {
            let result = get(server, &format!("task/{}", context.task.id)).await?;
            let task: TaskDetails =
                serde_json::from_value(result).context("Failed to parse task")?;

            done = task.state != State::Running;

            if done {
                match task.state {
                    State::Finished => {
                        if task.last_exit_code.is_none() || task.last_exit_code == Some(0) {
                            ui.set_status("Task completed successfully", Status::Succeeded);
                        } else {
                            ui.set_status("Task failed", Status::Failed);
                            if let Some(exit_code) = task.last_exit_code {
                                ui.eprintln(format!("Exit code: {}", exit_code).red().to_string());
                            }
                        }
                    }
                    State::Failed => {
                        ui.set_status("Task failed", Status::Failed);
                        // A task can fail without a failing subprocess; do not
                        // show a misleading "Exit code: 0".
                        if let Some(exit_code) = task.last_exit_code.filter(|c| *c != 0) {
                            ui.eprintln(format!("Exit code: {}", exit_code).red().to_string());
                        }
                    }
                    State::Running => {} // Should not happen
                }

                // The server may fail a task without a failing subprocess (e.g. a
                // panic or a failed app-data refresh), so the state is authoritative.
                if task.state == State::Failed || task.last_exit_code.is_some_and(|c| c != 0) {
                    return Err(match task.last_exit_code {
                        Some(c) if c != 0 => anyhow::anyhow!("Task failed with exit code {}", c),
                        _ => anyhow::anyhow!("Task failed"),
                    });
                }
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }
    }

    Ok(())
}

fn display_task_output_line(line: &scotty_types::OutputLine, ui: &Arc<Ui>) {
    use scotty_types::OutputStreamType;

    // Trim trailing newline since ui.println adds one
    let content = line.content.trim_end_matches('\n');

    let formatted_line = if ui.is_terminal() {
        match line.stream {
            OutputStreamType::Stdout => format!("    {}", content),
            OutputStreamType::Stderr => format!("    {}", content.dimmed()),
            OutputStreamType::Status => format!(" →  {}", content.blue()),
            OutputStreamType::StatusError => format!(" ✗  {}", content.red().bold()),
            OutputStreamType::Progress => format!(" ⋯  {}", content.yellow()),
            OutputStreamType::Info => format!(" •  {}", content.cyan()),
        }
    } else {
        // Non-terminal output: use text prefixes without colors
        match line.stream {
            OutputStreamType::Stdout => format!("    {}", content),
            OutputStreamType::Stderr => format!("    {}", content),
            OutputStreamType::Status => format!(" >  {}", content),
            OutputStreamType::StatusError => format!(" !  {}", content),
            OutputStreamType::Progress => format!(" ~  {}", content),
            OutputStreamType::Info => format!(" -  {}", content),
        }
    };

    ui.println(formatted_line);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn unauthorized() -> RetryError {
        RetryError::NonRetriable(HttpError::http(401, "Unauthorized"))
    }

    /// A server that accepts only `good`.
    async fn call(calls: &AtomicUsize, token: String) -> Result<String, RetryError> {
        calls.fetch_add(1, Ordering::SeqCst);
        if token == "good" {
            Ok(token)
        } else {
            Err(unauthorized())
        }
    }

    #[tokio::test]
    async fn a_401_refreshes_once_and_retries_once() {
        let calls = AtomicUsize::new(0);
        let refreshes = AtomicUsize::new(0);
        let result = send_with_refresh(
            "expired".to_string(),
            |t| call(&calls, t),
            || async {
                refreshes.fetch_add(1, Ordering::SeqCst);
                Some("good".to_string())
            },
        )
        .await;
        assert_eq!(result.unwrap(), "good");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(refreshes.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_failed_refresh_keeps_the_401_and_does_not_retry() {
        let calls = AtomicUsize::new(0);
        let result = send_with_refresh(
            "expired".to_string(),
            |t| call(&calls, t),
            || async { None },
        )
        .await;
        assert_eq!(result.unwrap_err().status_code(), Some(401));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_still_rejected_retry_is_not_retried_again() {
        let calls = AtomicUsize::new(0);
        let result = send_with_refresh(
            "expired".to_string(),
            |t| call(&calls, t),
            || async { Some("also-bad".to_string()) },
        )
        .await;
        assert_eq!(result.unwrap_err().status_code(), Some(401));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn success_or_other_errors_skip_the_refresh() {
        let calls = AtomicUsize::new(0);
        let result = send_with_refresh(
            "good".to_string(),
            |t| call(&calls, t),
            || async { panic!("no refresh without a 401") },
        )
        .await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_url_normalization_with_trailing_slash() {
        // Test case for issue #470: Trailing slash in Scotty URL
        assert_eq!(
            normalize_url("https://scottyurl/", "api/v1/apps/list"),
            "https://scottyurl/api/v1/apps/list"
        );

        assert_eq!(
            normalize_url("https://scottyurl", "api/v1/apps/list"),
            "https://scottyurl/api/v1/apps/list"
        );

        assert_eq!(
            normalize_url("https://scottyurl/", "/api/v1/apps/list"),
            "https://scottyurl/api/v1/apps/list"
        );

        assert_eq!(
            normalize_url("https://scottyurl", "/api/v1/apps/list"),
            "https://scottyurl/api/v1/apps/list"
        );

        // Edge case: multiple trailing slashes
        assert_eq!(
            normalize_url("https://scottyurl///", "api/v1/apps/list"),
            "https://scottyurl/api/v1/apps/list"
        );

        assert_eq!(
            normalize_url("https://scottyurl", "///api/v1/apps/list"),
            "https://scottyurl/api/v1/apps/list"
        );

        // Edge case: URL with extra slash causing double slash issue (like in the bug report)
        assert_eq!(
            normalize_url("https://scottyurl/", "/api/v1/apps/list"),
            "https://scottyurl/api/v1/apps/list"
        );

        // This would have produced "https://scottyurl//api/v1/apps/list" before the fix
        assert_ne!(
            normalize_url("https://scottyurl/", "/api/v1/apps/list"),
            "https://scottyurl//api/v1/apps/list"
        );
    }

    #[tokio::test]
    async fn logout_posts_the_token_and_accepts_204() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/authenticated/logout"))
            .and(header("authorization", "Bearer good"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;
        let url = format!("{}/api/v1/authenticated/logout", server.uri());
        assert!(post_logout(&url, "good".into()).await.is_ok());
        let err = post_logout(&url, "stale".into()).await.unwrap_err();
        assert_eq!(err.status_code(), Some(404), "unmatched token is refused");
    }

    #[tokio::test]
    async fn logout_refreshes_once_on_401() {
        let calls = AtomicUsize::new(0);
        let result = send_with_refresh(
            "expired".to_string(),
            |token| call(&calls, token),
            || async { Some("good".to_string()) },
        )
        .await;
        assert!(result.is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
