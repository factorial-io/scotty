use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::info;

use crate::api::error::AppError;
use crate::app_state::SharedAppState;
use crate::apps::app_data::AppData;
use crate::apps::file_list::FileList;
use crate::state_machine::StateHandler;
use crate::tasks::running_app_context::RunningAppContext;
use crate::{apps::app_data::AppSettings, state_machine::StateMachine};

use super::helper::run_sm;
use super::rebuild_app::rebuild_app_prepare;
use super::state_machine_handlers::context::Context;
use super::state_machine_handlers::create_directory_handler::CreateDirectoryHandler;
use super::state_machine_handlers::create_load_balancer_config::CreateLoadBalancerConfig;
use super::state_machine_handlers::save_files_handler::SaveFilesHandler;
use super::state_machine_handlers::save_settings_handler::SaveSettingsHandler;
use super::state_machine_handlers::set_finished_handler::SetFinishedHandler;
use super::state_machine_handlers::update_app_data_handler::UpdateAppDataHandler;

struct RunDockerComposeBuildHandler<S> {
    next_state: S,
    app: AppData,
}

#[async_trait::async_trait]
impl StateHandler<CreateAppStates, Context> for RunDockerComposeBuildHandler<CreateAppStates> {
    async fn transition(
        &self,
        _from: &CreateAppStates,
        context: Arc<RwLock<Context>>,
    ) -> anyhow::Result<CreateAppStates> {
        let sm = rebuild_app_prepare(&self.app).await?;
        let handle = sm.spawn(context.clone());
        let _ = handle.await;

        Ok(self.next_state)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum CreateAppStates {
    CreateDirectory,
    SaveSettings,
    SaveFiles,
    CreateLoadBalancerConfig,
    RunDockerComposeBuildAndRun,
    UpdateAppData,
    SetFinished,
    Done,
}

async fn create_app_prepare(
    app_state: SharedAppState,
    app: &AppData,
    settings: &AppSettings,
    files: &FileList,
) -> anyhow::Result<StateMachine<CreateAppStates, Context>> {
    let mut sm = StateMachine::new(CreateAppStates::CreateDirectory, CreateAppStates::Done);
    sm.add_handler(
        CreateAppStates::CreateDirectory,
        Arc::new(CreateDirectoryHandler::<CreateAppStates> {
            next_state: CreateAppStates::SaveSettings,
        }),
    );
    sm.add_handler(
        CreateAppStates::SaveSettings,
        Arc::new(SaveSettingsHandler::<CreateAppStates> {
            next_state: CreateAppStates::SaveFiles,
            settings: settings.clone(),
        }),
    );
    sm.add_handler(
        CreateAppStates::SaveFiles,
        Arc::new(SaveFilesHandler::<CreateAppStates> {
            next_state: CreateAppStates::CreateLoadBalancerConfig,
            files: files.clone(),
        }),
    );

    sm.add_handler(
        CreateAppStates::CreateLoadBalancerConfig,
        Arc::new(CreateLoadBalancerConfig::<CreateAppStates> {
            next_state: CreateAppStates::RunDockerComposeBuildAndRun,
            load_balancer_type: app_state.settings.load_balancer_type.clone(),
            settings: settings.clone(),
        }),
    );

    sm.add_handler(
        CreateAppStates::RunDockerComposeBuildAndRun,
        Arc::new(RunDockerComposeBuildHandler::<CreateAppStates> {
            next_state: CreateAppStates::UpdateAppData,
            app: app.clone(),
        }),
    );

    sm.add_handler(
        CreateAppStates::UpdateAppData,
        Arc::new(UpdateAppDataHandler::<CreateAppStates> {
            next_state: CreateAppStates::SetFinished,
        }),
    );

    sm.add_handler(
        CreateAppStates::SetFinished,
        Arc::new(SetFinishedHandler::<CreateAppStates> {
            next_state: CreateAppStates::Done,
        }),
    );
    Ok(sm)
}

fn validate_app(settings: &AppSettings, files: &FileList) -> anyhow::Result<()> {
    let docker_compose_file = files.files.iter().find(|f| {
        f.name.ends_with("docker-compose.yml") || f.name.ends_with("docker-compose.yaml")
    });

    if docker_compose_file.is_none() {
        return Err(AppError::NoDockerComposeFile.into());
    }
    // Parse docker-compose file
    let docker_compose_content = docker_compose_file.unwrap().content.clone();
    let docker_compose_data: serde_json::Value = serde_yml::from_slice(&docker_compose_content)
        .map_err(|_| AppError::InvalidDockerComposeFile)?;

    // Get list of available services
    let available_services: Vec<String> = docker_compose_data["services"]
        .as_object()
        .ok_or(AppError::InvalidDockerComposeFile)?
        .keys()
        .cloned()
        .collect();

    // Check if all public_services are available in docker-compose
    for public_service in &settings.public_services {
        if !available_services.contains(&public_service.service) {
            return Err(AppError::PublicServiceNotFound(public_service.service.clone()).into());
        }
    }

    Ok(())
}

pub async fn create_app(
    app_state: SharedAppState,
    app_name: &str,
    settings: &AppSettings,
    files: &FileList,
) -> anyhow::Result<RunningAppContext> {
    validate_app(settings, files)?;
    info!("Creating app: {}", app_name);
    let root_directory = app_state.settings.apps.root_folder.clone();
    let app_folder = slug::slugify(app_name);
    let root_directory = format!("{}/{}", root_directory, app_folder);

    // Check if files has a docker-compose file.
    // @TODO: Make more fool proof
    let candidate = files
        .files
        .iter()
        .find(|f| is_valid_docker_compose_file(&f.name));
    if candidate.is_none() {
        return Err(anyhow::anyhow!(
            "No docker-compose file found in provided files."
        ));
    }

    let docker_compose_path = format!("{}/{}", root_directory, candidate.unwrap().name);
    let app_data = AppData {
        name: app_name.to_string(),
        settings: Some(settings.clone()),
        services: vec![],
        docker_compose_path,
        root_directory,
        status: crate::apps::app_data::AppState::Creating,
    };
    let sm = create_app_prepare(app_state.clone(), &app_data, settings, files).await?;
    run_sm(app_state, &app_data, sm).await
}

fn is_valid_docker_compose_file(file_path: &str) -> bool {
    let file_name = std::path::Path::new(file_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    file_name == "docker-compose.yml" || file_name == "docker-compose.yaml"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_docker_compose_file() {
        assert!(is_valid_docker_compose_file("docker-compose.yml"));
        assert!(is_valid_docker_compose_file("docker-compose.yaml"));
        assert!(is_valid_docker_compose_file("./docker-compose.yaml"));
        assert!(is_valid_docker_compose_file("./docker-compose.yml"));
    }
}
