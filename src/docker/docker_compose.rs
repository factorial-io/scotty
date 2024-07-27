use std::path::{Path, PathBuf};

use tracing::instrument;

use crate::{app_state::SharedAppState, tasks::task_details::TaskDetails};

pub async fn run_docker_compose<F, Fut>(
    shared_app: &SharedAppState,
    docker_compose_path: &Path,
    command: &[&str],
    callback: F,
) -> anyhow::Result<TaskDetails>
where
    F: Fn() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + std::marker::Send,
{
    let manager = shared_app.task_manager.clone();

    let task_id = manager
        .start_process::<F, Fut>(
            docker_compose_path.parent().unwrap(),
            "docker-compose",
            command,
            callback,
        )
        .await;

    manager
        .get_task_details(&task_id)
        .await
        .ok_or(anyhow::Error::msg("Task not found"))
}

#[instrument]
pub fn run_docker_compose_now(
    docker_compose_path: &PathBuf,
    command: Vec<&str>,
) -> anyhow::Result<String> {
    let output = std::process::Command::new("docker-compose")
        .args(command)
        .current_dir(docker_compose_path.parent().unwrap())
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr).unwrap();
        return Err(anyhow::anyhow!(stderr));
    }

    let stdout = String::from_utf8(output.stdout).unwrap();

    Ok(stdout)
}
