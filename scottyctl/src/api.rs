use anyhow::Context;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde_json::Value;
use tracing::info;

use crate::auth::config::get_server_info;
use crate::auth::storage::TokenStorage;
use crate::context::ServerSettings;
use crate::utils::ui::Ui;
use owo_colors::OwoColorize;
use scotty_core::http::{HttpClient, RetryError};
use scotty_core::settings::api_server::AuthMode;
use scotty_core::tasks::running_app_context::RunningAppContext;
use scotty_core::tasks::task_details::{State, TaskDetails};
use scotty_core::version::VersionManager;
use std::sync::Arc;
use std::time::Duration;

pub async fn get_auth_token(server: &ServerSettings) -> Result<String, anyhow::Error> {
    // 1. Check server auth mode to determine if OAuth tokens should be used
    let server_supports_oauth = match get_server_info(server).await {
        Ok(server_info) => server_info.auth_mode == AuthMode::OAuth,
        Err(_) => false, // If we can't check, assume OAuth is not supported
    };

    // 2. Try stored OAuth token only if server supports OAuth
    if server_supports_oauth {
        if let Ok(Some(stored_token)) = TokenStorage::new()?.load_for_server(&server.server) {
            // TODO: Check if token is expired and refresh if needed
            return Ok(stored_token.access_token);
        }
    }

    // 3. Fall back to environment variable or command line token
    if let Some(token) = &server.access_token {
        return Ok(token.clone());
    }

    Err(anyhow::anyhow!(
        "No authentication available. Run 'scottyctl auth:login' or set SCOTTY_ACCESS_TOKEN"
    ))
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

pub async fn get_or_post(
    server: &ServerSettings,
    action: &str,
    method: &str,
    body: Option<Value>,
) -> anyhow::Result<Value> {
    let token = get_auth_token(server).await?;
    let url = format!("{}/api/v1/authenticated/{}", server.server, action);
    info!("Calling scotty API at {}", &url);

    let client = create_authenticated_client(&token)?;

    let result = match method.to_lowercase().as_str() {
        "post" => {
            if let Some(body) = body {
                client.post_json::<Value, Value>(&url, &body).await
            } else {
                client.post(&url, &serde_json::json!({})).await?;
                // For POST without body, we still need to get the response as JSON
                client.get_json::<Value>(&url).await
            }
        }
        "delete" => {
            if let Some(body) = body {
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
) -> anyhow::Result<()> {
    use crate::utils::status_line::Status;
    use crate::websocket::AuthenticatedWebSocket;
    use futures_util::{SinkExt, StreamExt};
    use scotty_core::tasks::task_details::TaskDetails;
    use scotty_core::websocket::message::{TaskOutputData, WebSocketMessage};
    use tokio_tungstenite::tungstenite::Message;

    // Try to connect to WebSocket for output streaming (optional enhancement)
    let ws_connection = AuthenticatedWebSocket::connect(server).await.ok();

    // If WebSocket connected, request task output stream
    if let Some(mut ws) = ws_connection {
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
        tracing::debug!("WebSocket connection failed, using polling only");

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

fn display_task_output_line(line: &scotty_core::output::OutputLine, ui: &Arc<Ui>) {
    use scotty_core::output::OutputStreamType;

    // Trim trailing newline since ui.println adds one
    let content = line.content.trim_end_matches('\n');

    let formatted_line = if ui.is_terminal() {
        match line.stream {
            OutputStreamType::Stdout => content.to_string(),
            OutputStreamType::Stderr => content.red().to_string(),
        }
    } else {
        content.to_string()
    };

    ui.println(formatted_line);
}
