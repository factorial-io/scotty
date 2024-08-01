use std::sync::Arc;

use tracing::{info, instrument};

use crate::{
    app_state::SharedAppState,
    apps::app_data::AppData,
    docker::state_machine_handlers::{
        Context, RunDockerComposeHandler, SetFinishedHandler, UpdateAppDataHandler,
    },
    state_machine::StateMachine,
    tasks::running_app_context::RunningAppContext,
};

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum StopAppStates {
    RunDockerCompose,
    UpdateAppData,
    SetFinished,
    Done,
}
#[instrument(skip(app_state))]
pub async fn stop_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
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
    let context = Context::create(app_state, app);

    let _ = sm.spawn(context.clone());

    Ok(context.clone().read().await.as_running_app_context().await)
}
