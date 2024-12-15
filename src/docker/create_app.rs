use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::info;

use crate::api::error::AppError;
use crate::app_state::SharedAppState;
use crate::apps::app_data::AppData;
use crate::apps::file_list::{File, FileList};
use crate::notification_types::{Message, MessageType};
use crate::settings::app_blueprint::ActionName;
use crate::state_machine::StateHandler;
use crate::tasks::running_app_context::RunningAppContext;
use crate::{apps::app_data::AppSettings, state_machine::StateMachine};

use super::helper::run_sm;
use super::rebuild_app::rebuild_app_prepare;
use super::state_machine_handlers::context::Context;
use super::state_machine_handlers::create_directory_handler::CreateDirectoryHandler;
use super::state_machine_handlers::create_load_balancer_config::CreateLoadBalancerConfig;
use super::state_machine_handlers::run_post_actions_handler::RunPostActionsHandler;
use super::state_machine_handlers::save_files_handler::SaveFilesHandler;
use super::state_machine_handlers::save_settings_handler::SaveSettingsHandler;
use super::state_machine_handlers::set_finished_handler::SetFinishedHandler;
use super::state_machine_handlers::update_app_data_handler::UpdateAppDataHandler;
use super::validation::validate_docker_compose_content;

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
        let app_state = &context.read().await.app_state;
        let sm = rebuild_app_prepare(app_state, &self.app, false).await?;
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
    RunPostActions,
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
            next_state: CreateAppStates::RunPostActions,
            app: app.clone(),
        }),
    );

    sm.add_handler(
        CreateAppStates::RunPostActions,
        Arc::new(RunPostActionsHandler::<CreateAppStates> {
            next_state: CreateAppStates::UpdateAppData,
            action: ActionName::PostCreate,
            settings: app.settings.clone(),
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
            notification: Some(Message::new(MessageType::AppCreated, app)),
        }),
    );
    Ok(sm)
}

fn validate_app(
    app_state: SharedAppState,
    settings: &AppSettings,
    files: &FileList,
) -> anyhow::Result<File> {
    let docker_compose_file = files
        .files
        .iter()
        .find(|f| is_valid_docker_compose_file(&f.name));

    if docker_compose_file.is_none() {
        return Err(AppError::NoDockerComposeFile.into());
    }
    // Parse docker-compose file
    let docker_compose_content = docker_compose_file.unwrap().content.clone();

    // Create a vector with all the public service names
    let public_service_names: Vec<String> = settings
        .public_services
        .iter()
        .map(|service| service.service.clone())
        .collect();

    let available_services =
        validate_docker_compose_content(&docker_compose_content, &public_service_names)?;
    // Check if we know about the private registry.
    if let Some(registry) = &settings.registry {
        if !app_state.settings.docker.registries.contains_key(registry) {
            return Err(AppError::RegistryNotFound(registry.clone()).into());
        }
    }

    if let Some(app_blueprint) = &settings.app_blueprint {
        if !app_state
            .settings
            .apps
            .blueprints
            .contains_key(app_blueprint)
        {
            return Err(AppError::AppBlueprintNotFound(app_blueprint.clone()).into());
        }

        let app_blueprint = &app_state.settings.apps.blueprints[app_blueprint];

        // Check if docker-compose services match required services
        let required_services = &app_blueprint.required_services;
        let missing_services: Vec<String> = required_services
            .iter()
            .filter(|service| !available_services.contains(service))
            .cloned()
            .collect();

        if !missing_services.is_empty() {
            return Err(AppError::AppBlueprintMismatch(format!(
                "docker compose does not contain all required services: {:?}",
                missing_services,
            ))
            .into());
        }
    }

    Ok(docker_compose_file.unwrap().clone())
}

pub async fn create_app(
    app_state: SharedAppState,
    app_name: &str,
    settings: &AppSettings,
    files: &FileList,
) -> anyhow::Result<RunningAppContext> {
    info!("Creating app: {}", app_name);
    let candidate = validate_app(app_state.clone(), settings, files)?;
    let root_directory = app_state.settings.apps.root_folder.clone();
    let app_folder = slug::slugify(app_name);
    let root_directory = format!("{}/{}", root_directory, app_folder);

    let docker_compose_path = format!("{}/{}", root_directory, candidate.name);
    let app_data = AppData {
        name: app_name.to_string(),
        settings: Some(settings.clone()),
        services: vec![],
        docker_compose_path,
        root_directory,
        status: crate::apps::app_data::AppStatus::Creating,
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
