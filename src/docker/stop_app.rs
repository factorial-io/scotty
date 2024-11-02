use std::sync::Arc;

use tracing::{info, instrument};

use crate::{
    api::error::AppError,
    app_state::SharedAppState,
    apps::app_data::{AppData, AppStatus},
    docker::state_machine_handlers::{
        context::Context, run_docker_compose_handler::RunDockerComposeHandler,
        set_finished_handler::SetFinishedHandler, update_app_data_handler::UpdateAppDataHandler,
    },
    state_machine::StateMachine,
    tasks::running_app_context::RunningAppContext,
};

use super::helper::run_sm;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum StopAppStates {
    RunDockerCompose,
    UpdateAppData,
    SetFinished,
    Done,
}
#[instrument]
pub async fn stop_app_prepare(
    app: &AppData,
) -> anyhow::Result<StateMachine<StopAppStates, Context>> {
    info!("Stopping app {} at {}", app.name, &app.docker_compose_path);

    let mut sm = StateMachine::new(StopAppStates::RunDockerCompose, StopAppStates::Done);

    sm.add_handler(
        StopAppStates::RunDockerCompose,
        Arc::new(RunDockerComposeHandler::<StopAppStates> {
            next_state: StopAppStates::UpdateAppData,
            command: ["stop"].iter().map(|s| s.to_string()).collect(),
        }),
    );
    sm.add_handler(
        StopAppStates::UpdateAppData,
        Arc::new(UpdateAppDataHandler::<StopAppStates> {
            next_state: StopAppStates::SetFinished,
        }),
    );
    sm.add_handler(
        StopAppStates::SetFinished,
        Arc::new(SetFinishedHandler::<StopAppStates> {
            next_state: StopAppStates::Done,
        }),
    );

    Ok(sm)
}

#[instrument(skip(app_state))]
pub async fn stop_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
    if app.status == AppStatus::Unsupported {
        return Err(AppError::OperationNotSupportedForLegacyApp(app.name.clone()).into());
    }
    let sm = stop_app_prepare(app).await?;
    run_sm(app_state, app, sm).await
}

#[instrument(skip(app_state))]
pub async fn force_stop_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
    let sm = stop_app_prepare(app).await?;
    run_sm(app_state, app, sm).await
}
