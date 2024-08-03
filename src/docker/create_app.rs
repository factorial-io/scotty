use std::sync::Arc;

use crate::app_state::SharedAppState;
use crate::apps::app_data::AppData;
use crate::apps::file_list::FileList;
use crate::tasks::running_app_context::RunningAppContext;
use crate::{apps::app_data::AppSettings, state_machine::StateMachine};

use super::state_machine_handlers::context::Context;
use super::state_machine_handlers::create_directory_handler::CreateDirectoryHandler;
use super::state_machine_handlers::save_files_handler::SaveFilesHandler;
use super::state_machine_handlers::save_settings_handler::SaveSettingsHandler;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum CreateAppStates {
    CreateDirectory,
    SaveSettings,
    SaveFiles,
    CreateLoadBalancerConfig,
    RunDockerComposeBuild,
    RunDockerComposeUp,
    UpdateAppData,
    SetFinished,
}

pub async fn create_app(
    app_state: SharedAppState,
    app_name: &str,
    settings: &AppSettings,
    files: &FileList,
) -> anyhow::Result<RunningAppContext> {
    let mut sm = StateMachine::new(
        CreateAppStates::CreateDirectory,
        CreateAppStates::SetFinished,
    );
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

    let root_directory = app_state.settings.apps.root_folder.clone();
    let app_folder = slug::slugify(app_name);
    let root_directory = format!("{}/{}", root_directory, app_folder);
    let docker_compose_path = format!("{}/{}", root_directory, app_name);

    let app_data = AppData {
        name: app_name.to_string(),
        settings: Some(settings.clone()),
        services: vec![],
        docker_compose_path,
        root_directory,
        status: crate::apps::app_data::AppState::Creating,
    };

    let context = Context::create(app_state, &app_data);
    let _ = sm.spawn(context.clone());

    Ok(context.clone().read().await.as_running_app_context().await)
}
