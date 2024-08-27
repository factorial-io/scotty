use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::sync::RwLock;
use tracing::instrument;

use crate::{app_state::SharedAppState, tasks::task_details::TaskDetails};

pub async fn run_task(
    shared_app: &SharedAppState,
    docker_compose_path: &Path,
    command: &str,
    args: &[&str],
    task: Arc<RwLock<TaskDetails>>,
) -> anyhow::Result<TaskDetails> {
    let manager = shared_app.task_manager.clone();

    let task_id = manager
        .start_process(
            docker_compose_path.parent().unwrap(),
            command,
            args,
            task.clone(),
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
