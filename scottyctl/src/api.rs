use anyhow::Context;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde_json::Value;
use tokio::time::sleep;
use tracing::{error, info};

use crate::auth::cache::CachedTokenManager;
use crate::auth::config::get_server_info;
use crate::context::ServerSettings;
use crate::utils::ui::Ui;
use owo_colors::OwoColorize;
use scotty_core::http::{HttpClient, RetryError};
use scotty_core::settings::api_server::AuthMode;
use scotty_core::tasks::running_app_context::RunningAppContext;
use scotty_core::tasks::task_details::State;
use scotty_core::version::VersionManager;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

const MAX_RETRIES: u8 = 3;
const INITIAL_RETRY_DELAY_MS: u64 = 100;
const MAX_RETRY_DELAY_MS: u64 = 2000;

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
}

/// Helper function to normalize URLs by handling trailing slashes
fn normalize_url(base_url: &str, path: &str) -> String {
    let mut normalized_base = base_url.trim_end_matches('/').to_string();
    let normalized_path = path.trim_start_matches('/');

    normalized_base.push('/');
    normalized_base.push_str(normalized_path);
    normalized_base
}

/// Helper function to determine if an error is retriable
#[allow(dead_code)]
fn is_retriable_error(err: &reqwest::Error) -> bool {
    err.is_timeout()
        || err.is_connect()
        || err.is_request()
        || err.status().is_some_and(|s| s.is_server_error())
}

/// Helper function to retry async operations with exponential backoff
async fn with_retry<F, Fut, T>(f: F) -> anyhow::Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let mut retry_count = 0;
    let mut delay = INITIAL_RETRY_DELAY_MS;

    loop {
        match f().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                // Check if we've reached the max retries
                if retry_count >= MAX_RETRIES - 1 {
                    return Err(err.context("Exhausted all retry attempts"));
                }

                // Check if it's a reqwest error that we should retry
                let should_retry = if let Some(reqwest_err) = err.downcast_ref::<reqwest::Error>() {
                    is_retriable_error(reqwest_err)
                } else {
                    // Also retry on JSON parsing errors which might be due to partial responses
                    err.to_string().contains("Failed to parse")
                };

                if !should_retry {
                    return Err(err);
                }

                retry_count += 1;
                error!(
                    "API call failed (attempt {}/{}), retrying in {}ms: {}",
                    retry_count, MAX_RETRIES, delay, err
                );

                // Sleep with exponential backoff
                sleep(Duration::from_millis(delay)).await;

                // Increase delay for next retry with exponential backoff (2x)
                // but cap it at MAX_RETRY_DELAY_MS
                delay = (delay * 2).min(MAX_RETRY_DELAY_MS);
            }
        }
    }
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

    // 3. Try stored OAuth token only if server supports OAuth
    if server_supports_oauth {
        if let Ok(Some(stored_token)) = get_cached_token_manager().load_for_server(&server.server) {
            // TODO: Check if token is expired and refresh if needed
            return Ok(stored_token.access_token);
        }
    }

    Err(anyhow::anyhow!(
        "No authentication available. Run 'scottyctl auth:login' or set SCOTTY_ACCESS_TOKEN"
    ))
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

    with_retry(|| async {
        let client = create_authenticated_client(&token)?;

        let result = match method.to_lowercase().as_str() {
            "post" => {
                if let Some(body) = body.clone() {
                    client.post_json::<Value, Value>(&url, &body).await
                } else {
                    client.post(&url, &serde_json::json!({})).await?;
                    // For POST without body, we still need to get the response as JSON
                    client.get_json::<Value>(&url).await
                }
            }
            "delete" => {
                if let Some(body) = body.clone() {
                    let response = client
                        .request_with_body(reqwest::Method::DELETE, &url, &body)
                        .await?;
                    response
                        .json::<Value>()
                        .await
                        .map_err(|e| RetryError::NonRetriable(e.into()))
                } else {
                    let response = client.request(reqwest::Method::DELETE, &url).await?;
                    response
                        .json::<Value>()
                        .await
                        .map_err(|e| RetryError::NonRetriable(e.into()))
                }
            }
            _ => client.get_json::<Value>(&url).await,
        };

        match result {
            Ok(value) => Ok(value),
            Err(RetryError::NonRetriable(err)) => {
                // Check if this is an HTTP error we can extract more info from
                if let Some(reqwest_err) = err.downcast_ref::<reqwest::Error>() {
                    if let Some(status) = reqwest_err.status() {
                        if status.is_client_error() {
                            return Err(anyhow::anyhow!(
                                "Client error calling scotty API at {}: {}",
                                &url,
                                status
                            ));
                        }
                    }
                }
                Err(err.context(format!("Failed to call scotty API at {}", &url)))
            }
            Err(RetryError::ExhaustedRetries(err)) => Err(err.context(format!(
                "Failed to call scotty API at {} after retries",
                &url
            ))),
        }
    })
    .await
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
            let result = get(server, &format!("task/{}", &context.task.id)).await?;
            let task: TaskDetails =
                serde_json::from_value(result).context("Failed to parse task")?;

            // Check if task is done
            done = task.state != State::Running;

            if done {
                // Abort the WebSocket handler since task is done
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
                        if let Some(exit_code) = task.last_exit_code {
                            ui.eprintln(format!("Exit code: {}", exit_code).red().to_string());
                        }
                    }
                    State::Running => {} // Should not happen since we check above
                }

                // Return error if task failed
                if let Some(exit_code) = task.last_exit_code {
                    if exit_code != 0 {
                        return Err(anyhow::anyhow!("Task failed with exit code {}", exit_code));
                    }
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
            let result = get(server, &format!("task/{}", &context.task.id)).await?;
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
                        if let Some(exit_code) = task.last_exit_code {
                            ui.eprintln(format!("Exit code: {}", exit_code).red().to_string());
                        }
                    }
                    State::Running => {} // Should not happen
                }

                if let Some(exit_code) = task.last_exit_code {
                    if exit_code != 0 {
                        return Err(anyhow::anyhow!("Task failed with exit code {}", exit_code));
                    }
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
}
