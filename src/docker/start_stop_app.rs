use std::path::{Path, PathBuf};

use tracing::{info, instrument};

use crate::{
    app_state::SharedAppState,
    apps::app_data::AppData,
    tasks::{task_details::TaskDetails, task_with_app_data::TaskWithAppData},
};

use super::{docker_compose::run_docker_compose, find_apps::inspect_app};

async fn update_app_data(
    app_state: &SharedAppState,
    docker_compose_path: &PathBuf,
) -> anyhow::Result<AppData> {
    let app_data = inspect_app(app_state, docker_compose_path).await?;
    app_state.apps.update_app(app_data).await
}

async fn run_docker_compose_impl(
    app_state: &SharedAppState,
    docker_compose_path: &Path,
    args: &[&str],
) -> anyhow::Result<TaskDetails> {
    let state = app_state.clone();
    let path = docker_compose_path.to_path_buf();
    let task = run_docker_compose(app_state, docker_compose_path, args, move || {
        let app_state = state.clone();
        let docker_compose_path = path.clone();
        async move {
            let app_state = app_state.clone();
            let docker_compose_path = docker_compose_path.clone();
            let _ = update_app_data(&app_state, &docker_compose_path).await;
        }
    })
    .await?;
    Ok(task)
}

#[instrument(skip(app_state))]
pub async fn run_app(app_state: SharedAppState, app: &AppData) -> anyhow::Result<TaskWithAppData> {
    info!("Running app {} at {}", app.name, &app.docker_compose_path);
    let docker_compose_path = std::path::PathBuf::from(&app.docker_compose_path);
    let task = run_docker_compose_impl(&app_state, &docker_compose_path, &["up", "-d"]).await?;
    let app_data = inspect_app(&app_state, &docker_compose_path).await?;
    let app_data = app_state.apps.update_app(app_data).await?;
    Ok(TaskWithAppData::new(app_data, task))
}

#[instrument(skip(app_state))]
pub async fn stop_app(app_state: SharedAppState, app: &AppData) -> anyhow::Result<TaskWithAppData> {
    info!("Stopping app {} at {}", app.name, &app.docker_compose_path);
    let docker_compose_path = std::path::PathBuf::from(&app.docker_compose_path);
    let task = run_docker_compose_impl(&app_state, &docker_compose_path, &["stop"]).await?;

    let app_data = inspect_app(&app_state, &docker_compose_path).await?;
    let app_data = app_state.apps.update_app(app_data).await?;
    Ok(TaskWithAppData::new(app_data, task))
}

#[instrument(skip(app_state))]
pub async fn rm_app(app_state: SharedAppState, app: &AppData) -> anyhow::Result<TaskWithAppData> {
    info!(
        "Removing docker related data for app {} at {}",
        app.name, &app.docker_compose_path
    );
    let docker_compose_path = std::path::PathBuf::from(&app.docker_compose_path);
    let task =
        run_docker_compose_impl(&app_state, &docker_compose_path, &["rm", "-s", "-f"]).await?;

    let app_data = inspect_app(&app_state, &docker_compose_path).await?;
    let app_data = app_state.apps.update_app(app_data).await?;

    Ok(TaskWithAppData::new(app_data, task))
}
