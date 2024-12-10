use std::sync::Arc;

use tracing::{info, instrument};

use crate::{
    api::error::AppError,
    app_state::SharedAppState,
    apps::app_data::{AppData, AppStatus},
    docker::state_machine_handlers::{
        context::Context, run_docker_compose_handler::RunDockerComposeHandler,
        run_post_actions_handler::RunPostActionsHandler, set_finished_handler::SetFinishedHandler,
        update_app_data_handler::UpdateAppDataHandler,
    },
    notification_types::{Message, MessageType},
    settings::app_blueprint::ActionName,
    state_machine::StateMachine,
    tasks::running_app_context::RunningAppContext,
};

use super::helper::run_sm;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum RunAppStates {
    RunDockerCompose,
    RunPostActions,
    UpdateAppData,
    SetFinished,
    Done,
}
#[instrument()]
async fn run_app_prepare(app: &AppData) -> anyhow::Result<StateMachine<RunAppStates, Context>> {
    info!("Running app {} at {}", app.name, &app.docker_compose_path);

    let mut sm = StateMachine::new(RunAppStates::RunDockerCompose, RunAppStates::Done);

    sm.add_handler(
        RunAppStates::RunDockerCompose,
        Arc::new(RunDockerComposeHandler::<RunAppStates> {
            next_state: RunAppStates::RunPostActions,
            command: ["up", "-d"].iter().map(|s| s.to_string()).collect(),
            env: app.get_environment(),
        }),
    );
    sm.add_handler(
        RunAppStates::RunPostActions,
        Arc::new(RunPostActionsHandler::<RunAppStates> {
            next_state: RunAppStates::UpdateAppData,
            action: ActionName::PostRun,
            settings: app.settings.clone(),
        }),
    );
    sm.add_handler(
        RunAppStates::UpdateAppData,
        Arc::new(UpdateAppDataHandler::<RunAppStates> {
            next_state: RunAppStates::SetFinished,
        }),
    );
    sm.add_handler(
        RunAppStates::SetFinished,
        Arc::new(SetFinishedHandler::<RunAppStates> {
            next_state: RunAppStates::Done,
            notification: Some(Message::new(MessageType::AppStarted, app)),
        }),
    );

    Ok(sm)
}

#[instrument(skip(app_state))]
pub async fn run_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
    if app.status == AppStatus::Unsupported {
        return Err(AppError::OperationNotSupportedForLegacyApp(app.name.clone()).into());
    }

    let sm = run_app_prepare(app).await?;
    run_sm(app_state, app, sm).await
}
