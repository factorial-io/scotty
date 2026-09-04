use std::path::Path;

use scotty_core::utils::secret::SecretHashMap;
use tracing::{error, info};

use crate::docker::docker_compose::run_task;

use super::context::Context;

/// Run a subprocess as one step of the current task and wait for it.
/// A non-zero exit, a signal, or a spawn failure is an error for the caller;
/// the task itself stays `Running` until its owner terminates it.
pub async fn run_task_and_wait(
    context: &Context,
    docker_compose_path: &Path,
    command: &str,
    args: &[&str],
    env: &SecretHashMap,
    msg: &str,
) -> anyhow::Result<()> {
    info!(
        app_name = %context.app_data.name,
        command = %command,
        args = ?args,
        "Starting task: {}", msg
    );
    let writer = context.task.writer();

    let handle = run_task(
        &context.app_state,
        docker_compose_path,
        command,
        args,
        env,
        writer.clone(),
    )
    .await?;

    let failure = match handle.await {
        Ok(Ok(0)) => None,
        Ok(Ok(code)) => Some(format!("{msg} (exit code {code})")),
        Ok(Err(e)) => Some(format!("{msg}: {e:#}")),
        Err(join) => Some(format!("{msg}: process task {join}")),
    };

    if let Some(reason) = failure {
        writer.status_error(format!("Failed: {reason}")).await;
        error!(
            app_name = %context.app_data.name,
            command = %command,
            args = ?args,
            "Task failed: {}", reason
        );
        return Err(anyhow::anyhow!("{reason}"));
    }

    writer.status(format!("Completed: {}", msg)).await;
    info!(
        app_name = %context.app_data.name,
        command = %command,
        "Task completed successfully: {}", msg
    );
    Ok(())
}
