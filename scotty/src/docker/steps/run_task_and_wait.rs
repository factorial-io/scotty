use std::path::Path;

use scotty_core::utils::secret::{MaskedSecret, SecretHashMap};
use tracing::{error, info};

use crate::docker::docker_compose::run_task;
use crate::tasks::manager::OutputMode;

use super::context::Context;

/// Run a subprocess as one step of the current task and wait for it.
/// A non-zero exit, a signal, or a spawn failure is an error for the caller;
/// the task itself stays `Running` until its owner terminates it.
#[allow(clippy::too_many_arguments)]
pub async fn run_task_and_wait(
    context: &Context,
    docker_compose_path: &Path,
    command: &str,
    args: &[&str],
    env: &SecretHashMap,
    stdin: Option<&MaskedSecret>,
    msg: &str,
    mode: OutputMode,
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
        stdin,
        writer.clone(),
        mode,
    )
    .await?;

    // Aborts the process task (and with it the process) if this step is
    // cancelled, e.g. when the operation exceeds its maximum duration.
    struct AbortOnDrop<T>(tokio::task::JoinHandle<T>);
    impl<T> Drop for AbortOnDrop<T> {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let mut handle = AbortOnDrop(handle);

    let failure = match (&mut handle.0).await {
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
