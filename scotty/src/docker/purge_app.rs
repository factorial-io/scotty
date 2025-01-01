use std::sync::Arc;

use tracing::{info, instrument};

use crate::{
    api::error::AppError,
    app_state::SharedAppState,
    docker::state_machine_handlers::{
        context::Context, run_docker_compose_handler::RunDockerComposeHandler,
        set_finished_handler::SetFinishedHandler, update_app_data_handler::UpdateAppDataHandler,
    },
    state_machine::StateMachine,
};
use scotty_core::apps::app_data::{AppData, AppStatus};
use scotty_core::notification_types::{Message, MessageType};
use scotty_core::tasks::running_app_context::RunningAppContext;

use super::helper::run_sm;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PurgeAppStates {
    RunDockerCompose,
    UpdateAppData,
    SetFinished,
    Done,
}

#[derive(Copy, Clone, Debug)]
pub enum PurgeAppMethod {
    Down,
    Rm,
}
#[instrument]
pub async fn purge_app_prepare(
    app: &AppData,
    purge_method: PurgeAppMethod,
) -> anyhow::Result<StateMachine<PurgeAppStates, Context>> {
    info!("Purging app {} at {}", app.name, &app.docker_compose_path);

    let mut sm = StateMachine::new(PurgeAppStates::RunDockerCompose, PurgeAppStates::Done);

    let command = match purge_method {
        PurgeAppMethod::Down => vec!["down", "-v", "--rmi", "all"],
        PurgeAppMethod::Rm => vec!["rm", "-s", "-f"],
    };

    sm.add_handler(
        PurgeAppStates::RunDockerCompose,
        Arc::new(RunDockerComposeHandler::<PurgeAppStates> {
            next_state: PurgeAppStates::UpdateAppData,
            command: command.iter().map(|s| s.to_string()).collect(),
            env: app.get_environment(),
        }),
    );
    sm.add_handler(
        PurgeAppStates::UpdateAppData,
        Arc::new(UpdateAppDataHandler::<PurgeAppStates> {
            next_state: PurgeAppStates::SetFinished,
        }),
    );
    sm.add_handler(
        PurgeAppStates::SetFinished,
        Arc::new(SetFinishedHandler::<PurgeAppStates> {
            next_state: PurgeAppStates::Done,
            notification: Some(Message::new(MessageType::AppPurged, app)),
        }),
    );
    Ok(sm)
}

#[instrument(skip(app_state))]
pub async fn purge_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
    if app.status == AppStatus::Unsupported {
        return Err(AppError::OperationNotSupportedForLegacyApp(app.name.clone()).into());
    }
    let sm = purge_app_prepare(app, PurgeAppMethod::Rm).await?;
    run_sm(app_state, app, sm).await
}
