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

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum RunAppStates {
    RunDockerCompose,
    UpdateAppData,
    SetFinished,
    Done,
}
#[instrument(skip(app_state))]
pub async fn run_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
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

    let context = Context::create(app_state, app);
    let _ = sm.spawn(context.clone());

    Ok(context.clone().read().await.as_running_app_context().await)
}
