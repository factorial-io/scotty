use std::path::Path;

use scotty_core::utils::secret::{MaskedSecret, SecretHashMap};
use tracing::instrument;

use crate::{
    app_state::SharedAppState,
    onepassword::lookup::resolve_environment_variables,
    tasks::{actor::TaskWriter, manager::OutputMode},
};

/// Runs a docker-compose command as a step of the task behind `writer`.
/// Resolves to the process exit code; the task state is left to its owner.
#[allow(clippy::too_many_arguments)]
pub async fn run_task(
    shared_app: &SharedAppState,
    docker_compose_path: &Path,
    command: &str,
    args: &[&str],
    env: &SecretHashMap,
    stdin: Option<&MaskedSecret>,
    writer: TaskWriter,
    mode: OutputMode,
) -> anyhow::Result<tokio::task::JoinHandle<anyhow::Result<i32>>> {
    // Resolve environment variables (1Password, substitutions, etc.)
    let resolved_environment = resolve_environment_variables(&shared_app.settings, env).await;

    let parent_dir = docker_compose_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Docker compose path has no parent directory"))?;

    let exposed_env = resolved_environment.expose_all();
    Ok(shared_app.task_manager.start_process(
        parent_dir,
        command,
        args,
        &exposed_env,
        stdin,
        writer,
        mode,
    ))
}

/// Runs a docker-compose command to completion and returns the output.
///
/// Async and `kill_on_drop`, so a caller's timeout (e.g. the app data refresh
/// after an operation) both frees the runtime worker and ends the process.
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
#[instrument(skip(env_vars))]
pub async fn run_docker_compose_now(
    docker_compose_path: &Path,
    command: &[&str],
    env_vars: Option<&std::collections::HashMap<String, String>>,
    return_stderr: bool,
) -> anyhow::Result<String> {
    let mut cmd = tokio::process::Command::new("docker-compose");
    cmd.kill_on_drop(true);

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
    let output = cmd.output().await?;

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
