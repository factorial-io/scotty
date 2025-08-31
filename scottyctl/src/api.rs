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

async fn get_auth_token(server: &ServerSettings) -> Result<String, anyhow::Error> {
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
                let response = client.request_with_body(reqwest::Method::DELETE, &url, &body).await?;
                response.json::<Value>().await.map_err(|e| RetryError::NonRetriable(e.into()))
            } else {
                let response = client.request(reqwest::Method::DELETE, &url).await?;
                response.json::<Value>().await.map_err(|e| RetryError::NonRetriable(e.into()))
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

pub async fn delete(server: &ServerSettings, method: &str, body: Option<Value>) -> anyhow::Result<Value> {
    get_or_post(server, method, "delete", body).await
}

pub async fn wait_for_task(
    server: &ServerSettings,
    context: &RunningAppContext,
    ui: &Arc<Ui>,
) -> anyhow::Result<()> {
    let mut done = false;
    let mut last_position = 0;
    let mut last_err_position = 0;

    while !done {
        let result = get(server, &format!("task/{}", &context.task.id)).await?;

        let task: TaskDetails = serde_json::from_value(result).context("Failed to parse task")?;

        // Handle stderr
        {
            let stderr = &task.stderr[last_err_position..];
            if let Some(last_newline_pos) = stderr.rfind('\n') {
                let mut partial_output = stderr[..=last_newline_pos].to_string();
                last_err_position += last_newline_pos + 1;

                // Remove the newline before printing
                partial_output.pop();
                ui.eprintln(partial_output.blue().to_string());
            }
        }
        // Handle stdout
        {
            let stdout = &task.stdout[last_position..];
            if let Some(last_newline_pos) = stdout.rfind('\n') {
                let mut partial_output = stdout[..=last_newline_pos].to_string();
                last_position += last_newline_pos + 1;

                // Remove the newline before printing
                partial_output.pop();
                ui.println(partial_output.blue().to_string());
            }
        }

        // Check if task is done
        done = task.state != State::Running;
        if !done {
            tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
        }

        if let Some(exit_code) = task.last_exit_code {
            if done && exit_code != 0 {
                return Err(anyhow::anyhow!("Task failed with exit code {}", exit_code));
            }
        }
    }

    Ok(())
}
