use anyhow::Context;
use serde_json::Value;
use tokio::time::{sleep, Duration};
use tracing::{error, info};

use crate::context::ServerSettings;
use crate::utils::ui::Ui;
use owo_colors::OwoColorize;
use std::sync::Arc;

use scotty_core::tasks::running_app_context::RunningAppContext;
use scotty_core::tasks::task_details::{State, TaskDetails};

// Constants for retry mechanism
const MAX_RETRIES: usize = 5;
const INITIAL_RETRY_DELAY_MS: u64 = 500;
const MAX_RETRY_DELAY_MS: u64 = 8000; // 8 seconds

/// Helper function to normalize URLs by handling trailing slashes
fn normalize_url(base_url: &str, path: &str) -> String {
    let mut normalized_base = base_url.trim_end_matches('/').to_string();
    let normalized_path = path.trim_start_matches('/');

    normalized_base.push('/');
    normalized_base.push_str(normalized_path);
    normalized_base
}

/// Helper function to determine if an error is retriable
fn is_retriable_error(err: &reqwest::Error) -> bool {
    err.is_timeout()
        || err.is_connect()
        || err.is_request()
        || err.status().is_some_and(|s| s.is_server_error())
}

/// Helper function to execute a future with retry logic
async fn with_retry<F, Fut, T>(f: F) -> anyhow::Result<T>
where
    F: Fn() -> Fut + Clone,
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

pub async fn get_or_post(
    server: &ServerSettings,
    action: &str,
    method: &str,
    body: Option<Value>,
) -> anyhow::Result<Value> {
    let url = normalize_url(&server.server, &format!("api/v1/{}", action));
    info!("Calling scotty API at {}", &url);

    with_retry(|| async {
        let client = reqwest::Client::new();
        let response = match method.to_lowercase().as_str() {
            "post" => {
                if let Some(body) = body.clone() {
                    client.post(&url).json(&body)
                } else {
                    client.post(&url)
                }
            }
            _ => client.get(&url),
        };

        let response = response
            .bearer_auth(server.access_token.as_deref().unwrap_or_default())
            .timeout(Duration::from_secs(10)) // Add timeout for requests
            .send()
            .await
            .context(format!("Failed to call scotty API at {}", &url))?;

        // Client errors (4xx) shouldn't be retried - fail fast
        if response.status().is_client_error() {
            let status = response.status();
            let content = response.json::<Value>().await.ok();
            let error_message = if let Some(content) = content {
                if let Some(message) = content.get("message") {
                    format!(": {}", message.as_str().unwrap_or("Unknown error"))
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            return Err(anyhow::anyhow!(
                "Client error calling scotty API at {} : {}{}",
                &url,
                &status,
                error_message
            ));
        }

        if response.status().is_success() {
            let json = response.json::<Value>().await.context(format!(
                "Failed to parse response from scotty API at {}",
                &url
            ))?;
            Ok(json)
        } else {
            let status = &response.status();
            let content = response.json::<Value>().await.ok();
            let error_message = if let Some(content) = content {
                if let Some(message) = content.get("message") {
                    format!(": {}", message.as_str().unwrap_or("Unknown error"))
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            Err(anyhow::anyhow!(
                "Failed to call scotty API at {} : {}{}",
                &url,
                &status,
                error_message
            ))
        }
    })
    .await
}

pub async fn get(server: &ServerSettings, method: &str) -> anyhow::Result<Value> {
    get_or_post(server, method, "GET", None).await
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
