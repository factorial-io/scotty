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
enum PurgeAppStates {
    RunDockerCompose,
    UpdateAppData,
    SetFinished,
    Done,
}
#[instrument]
async fn purge_app_prepare(app: &AppData) -> anyhow::Result<StateMachine<PurgeAppStates, Context>> {
    info!("Stopping app {} at {}", app.name, &app.docker_compose_path);

    let mut sm = StateMachine::new(PurgeAppStates::RunDockerCompose, PurgeAppStates::Done);

    sm.add_handler(
        PurgeAppStates::RunDockerCompose,
        Arc::new(RunDockerComposeHandler::<PurgeAppStates> {
            next_state: PurgeAppStates::UpdateAppData,
            command: ["rm", "-s", "-f"].iter().map(|s| s.to_string()).collect(),
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
        }),
    );
    Ok(sm)
}

#[instrument(skip(app_state))]
pub async fn purge_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
    let sm = purge_app_prepare(app).await?;
    run_sm(app_state, app, sm).await
}
