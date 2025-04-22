use std::{path::Path, sync::Arc};

use scotty_core::tasks::task_details::TaskDetails;
use tokio::sync::RwLock;
use tracing::instrument;

use crate::{app_state::SharedAppState, onepassword::lookup::resolve_environment_variables};

/// Runs a docker-compose command asynchronously as a task
///
/// # Arguments
///
/// * `shared_app` - The shared application state
/// * `docker_compose_path` - Path to the docker-compose file
/// * `command` - The main command to run
/// * `args` - Additional arguments for the command
/// * `env` - Environment variables to pass to the command
/// * `task` - Task details for tracking
///
/// # Returns
///
/// Task details after execution
pub async fn run_task(
    shared_app: &SharedAppState,
    docker_compose_path: &Path,
    command: &str,
    args: &[&str],
    env: &std::collections::HashMap<String, String>,
    task: Arc<RwLock<TaskDetails>>,
) -> anyhow::Result<TaskDetails> {
    let manager = shared_app.task_manager.clone();

    let resolved_environment = resolve_environment_variables(&shared_app.settings, env).await;

    let parent_dir = docker_compose_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Docker compose path has no parent directory"))?;

    let task_id = manager
        .start_process(
            parent_dir,
            command,
            args,
            &resolved_environment,
            task.clone(),
        )
        .await;

    manager
        .get_task_details(&task_id)
        .await
        .ok_or_else(|| anyhow::anyhow!("Task not found"))
}

/// Runs a docker-compose command synchronously and returns the output
///
/// # Arguments
///
/// * `docker_compose_path` - Path to the docker-compose file
/// * `command` - Command arguments to pass to docker-compose
/// * `env_vars` - Optional environment variables to pass to the command
/// * `return_stderr` - Whether to return stderr instead of stdout
///
/// # Returns
///
/// The command output as a string
#[instrument]
pub fn run_docker_compose_now(
    docker_compose_path: &Path,
    command: &[&str],
    env_vars: Option<&std::collections::HashMap<String, String>>,
    return_stderr: bool,
) -> anyhow::Result<String> {
    let mut cmd = std::process::Command::new("docker-compose");

    // Add args and set working directory
    let parent_dir = docker_compose_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Docker compose path has no parent directory"))?;

    cmd.args(command).current_dir(parent_dir);

    // Apply environment variables if provided
    if let Some(env_map) = env_vars {
        for (key, value) in env_map {
            cmd.env(key, value);
        }
    }

    // Execute the command
    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr)
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in stderr: {}", e))?;
        return Err(anyhow::anyhow!("Docker compose command failed: {}", stderr));
    }

    let out = match return_stderr {
        false => String::from_utf8(output.stdout)
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in stdout: {}", e))?,
        true => String::from_utf8(output.stderr)
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in stderr: {}", e))?,
    };

    Ok(out)
}
