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
enum RebuildAppStates {
    RunDockerComposePull,
    RunDockerComposeStop,
    RunDockerComposeRun,
    UpdateAppData,
    SetFinished,
    Done,
}
#[instrument(skip(app_state))]
pub async fn rebuild_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
    info!(
        "Rebuilding app {} at {}",
        app.name, &app.docker_compose_path
    );

    let mut sm = StateMachine::new(
        RebuildAppStates::RunDockerComposePull,
        RebuildAppStates::Done,
    );

    sm.add_handler(
        RebuildAppStates::RunDockerComposePull,
        Arc::new(RunDockerComposeHandler::<RebuildAppStates> {
            next_state: RebuildAppStates::RunDockerComposeStop,
            command: ["build", "--pull"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }),
    );
    sm.add_handler(
        RebuildAppStates::RunDockerComposeStop,
        Arc::new(RunDockerComposeHandler::<RebuildAppStates> {
            next_state: RebuildAppStates::RunDockerComposeRun,
            command: ["stop"].iter().map(|s| s.to_string()).collect(),
        }),
    );
    sm.add_handler(
        RebuildAppStates::RunDockerComposeRun,
        Arc::new(RunDockerComposeHandler::<RebuildAppStates> {
            next_state: RebuildAppStates::UpdateAppData,
            command: ["up", "-d"].iter().map(|s| s.to_string()).collect(),
        }),
    );
    sm.add_handler(
        RebuildAppStates::UpdateAppData,
        Arc::new(UpdateAppDataHandler::<RebuildAppStates> {
            next_state: RebuildAppStates::SetFinished,
        }),
    );
    sm.add_handler(
        RebuildAppStates::SetFinished,
        Arc::new(SetFinishedHandler::<RebuildAppStates> {
            next_state: RebuildAppStates::Done,
        }),
    );
    let context = Context::create(app_state, app);

    let _ = sm.spawn(context.clone());

    Ok(context.clone().read().await.as_running_app_context().await)
}
