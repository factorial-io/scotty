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
enum PurgeAppStates {
    RunDockerCompose,
    UpdateAppData,
    SetFinished,
    Done,
}
#[instrument(skip(app_state))]
pub async fn purge_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
    info!("Stopping app {} at {}", app.name, &app.docker_compose_path);

    let mut sm = StateMachine::new(PurgeAppStates::RunDockerCompose, PurgeAppStates::Done);

    sm.add_handler(
        PurgeAppStates::RunDockerCompose,
        Arc::new(RunDockerComposeHandler::<PurgeAppStates> {
            next_state: PurgeAppStates::UpdateAppData,
            command: ["rm", "-s", "-f"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
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
    let context = Context::create(app_state, app);

    let _ = sm.spawn(context.clone());

    Ok(context.clone().read().await.as_running_app_context().await)
}
