use std::sync::Arc;

use tracing::{info, instrument};

use crate::{
    app_state::SharedAppState,
    apps::app_data::AppData,
    docker::state_machine_handlers::{
        context::Context, run_docker_compose_handler::RunDockerComposeHandler,
        set_finished_handler::SetFinishedHandler, update_app_data_handler::UpdateAppDataHandler,
    },
    state_machine::StateMachine,
    tasks::running_app_context::RunningAppContext,
};

use super::helper::run_sm;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum RunAppStates {
    RunDockerCompose,
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
            next_state: RunAppStates::UpdateAppData,
            command: ["up", "-d"].iter().map(|s| s.to_string()).collect(),
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
        }),
    );

    Ok(sm)
}

#[instrument(skip(app_state))]
pub async fn run_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
    let sm = run_app_prepare(app).await?;
    run_sm(app_state, app, sm).await
}
