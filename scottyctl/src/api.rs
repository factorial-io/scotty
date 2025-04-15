use anyhow::Context;
use serde_json::Value;
use tracing::info;

use crate::ServerSettings;
use owo_colors::OwoColorize;
use scotty_core::tasks::running_app_context::RunningAppContext;
use scotty_core::tasks::task_details::{State, TaskDetails};

pub async fn get_or_post(
    server: &ServerSettings,
    action: &str,
    method: &str,
    body: Option<Value>,
) -> anyhow::Result<Value> {
    let url = format!("{}/api/v1/{}", server.server, action);
    info!("Calling scotty API at {}", &url);

    let client = reqwest::Client::new();
    let response = match method.to_lowercase().as_str() {
        "post" => {
            if let Some(body) = body {
                client.post(&url).json(&body)
            } else {
                client.post(&url)
            }
        }
        _ => client.get(&url),
    };

    let response = response
        .bearer_auth(server.access_token.as_deref().unwrap_or_default())
        .send()
        .await
        .context(format!("Failed to call scotty API at {}", &url))?;

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
}

pub async fn get(server: &ServerSettings, method: &str) -> anyhow::Result<Value> {
    get_or_post(server, method, "GET", None).await
}

pub async fn wait_for_task(
    server: &ServerSettings,
    context: &RunningAppContext,
) -> anyhow::Result<()> {
    let mut done = false;
    let mut last_position = 0;
    let mut last_err_position = 0;

    while !done {
        let result = get(server, &format!("task/{}", &context.task.id)).await?;

        let task: TaskDetails = serde_json::from_value(result).context("Failed to parse task")?;

        // Handle stderr
        {
            let partial_output = task.stderr[last_err_position..].to_string();
            last_err_position = task.stderr.len();
            eprint!("{}", partial_output.blue());
        }
        // Handle stdout
        {
            let partial_output = task.stdout[last_position..].to_string();
            last_position = task.stdout.len();
            print!("{}", partial_output.blue());
        }

        // Check if task is done
        done = task.state != State::Running;
        if !done {
            // Sleep for half a second
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        if let Some(exit_code) = task.last_exit_code {
            if done && exit_code != 0 {
                return Err(anyhow::anyhow!("Task failed with exit code {}", exit_code));
            }
        }
    }

    Ok(())
}
