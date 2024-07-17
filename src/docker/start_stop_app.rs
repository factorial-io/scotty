use tracing::{info, instrument};

use crate::{app_state::SharedAppState, apps::app_data::AppData};

use super::{docker_compose::run_docker_compose, find_apps::inspect_app};

#[instrument(skip(app_state))]
pub async fn run_app(app_state: SharedAppState, app: &AppData) -> anyhow::Result<AppData> {
    info!("Running app {} at {}", app.name, &app.docker_compose_path);
    let docker_compose_path = std::path::PathBuf::from(&app.docker_compose_path);
    run_docker_compose(&docker_compose_path, vec!["up", "-d"])?;

    let app_data = inspect_app(&app_state, docker_compose_path).await?;
    app_state.apps.update_app(app_data).await
}

#[instrument(skip(app_state))]
pub async fn stop_app(app_state: SharedAppState, app: &AppData) -> anyhow::Result<AppData> {
    info!("Stopping app {} at {}", app.name, &app.docker_compose_path);
    let docker_compose_path = std::path::PathBuf::from(&app.docker_compose_path);
    run_docker_compose(&docker_compose_path, vec!["stop"])?;

    let app_data = inspect_app(&app_state, docker_compose_path).await?;
    app_state.apps.update_app(app_data).await
}

#[instrument(skip(app_state))]
pub async fn rm_app(app_state: SharedAppState, app: &AppData) -> anyhow::Result<AppData> {
    info!(
        "Removing docker related data for app {} at {}",
        app.name, &app.docker_compose_path
    );
    let docker_compose_path = std::path::PathBuf::from(&app.docker_compose_path);
    run_docker_compose(&docker_compose_path, vec!["rm", "-s", "-f"])?;

    let app_data = inspect_app(&app_state, docker_compose_path).await?;
    app_state.apps.update_app(app_data).await
}
