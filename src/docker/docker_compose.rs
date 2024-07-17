use std::{path::PathBuf, process::Command};

use tracing::instrument;

#[instrument]
pub fn run_docker_compose(
    docker_compose_path: &PathBuf,
    command: Vec<&str>,
) -> anyhow::Result<String> {
    let output = Command::new("docker-compose")
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
