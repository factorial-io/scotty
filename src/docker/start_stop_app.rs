use tracing::{info, instrument};

use crate::{
    app_state::SharedAppState, apps::app_data::AppData, tasks::task_with_app_data::TaskWithAppData,
};

use super::{docker_compose::run_docker_compose, find_apps::inspect_app};

#[instrument(skip(app_state))]
pub async fn run_app(app_state: SharedAppState, app: &AppData) -> anyhow::Result<TaskWithAppData> {
    info!("Running app {} at {}", app.name, &app.docker_compose_path);
    let docker_compose_path = std::path::PathBuf::from(&app.docker_compose_path);
    let task = run_docker_compose(&app_state, &docker_compose_path, vec!["up", "-d"]).await?;

    let app_data = inspect_app(&app_state, &docker_compose_path).await?;
    let app_data = app_state.apps.update_app(app_data).await?;
    Ok(TaskWithAppData::new(app_data, task))
}

#[instrument(skip(app_state))]
pub async fn stop_app(app_state: SharedAppState, app: &AppData) -> anyhow::Result<TaskWithAppData> {
    info!("Stopping app {} at {}", app.name, &app.docker_compose_path);
    let docker_compose_path = std::path::PathBuf::from(&app.docker_compose_path);
    let task = run_docker_compose(&app_state, &docker_compose_path, vec!["stop"]).await?;

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
    let task = run_docker_compose(&app_state, &docker_compose_path, vec!["rm", "-s", "-f"]).await?;

    let app_data = inspect_app(&app_state, &docker_compose_path).await?;
    let app_data = app_state.apps.update_app(app_data).await?;

    Ok(TaskWithAppData::new(app_data, task))
}
