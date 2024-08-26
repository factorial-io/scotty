use std::sync::Arc;

use tracing::{info, instrument};

use crate::{
    app_state::SharedAppState,
    apps::app_data::AppData,
    docker::state_machine_handlers::{
        context::Context, run_docker_compose_handler::RunDockerComposeHandler,
        run_docker_login_handler::RunDockerLoginHandler, set_finished_handler::SetFinishedHandler,
        update_app_data_handler::UpdateAppDataHandler,
    },
    state_machine::StateMachine,
    tasks::running_app_context::RunningAppContext,
};

use super::helper::run_sm;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum RebuildAppStates {
    RunDockerLogin,
    RunDockerComposePull,
    RunDockerComposeStop,
    RunDockerComposeRun,
    UpdateAppData,
    SetFinished,
    Done,
}

#[instrument]
pub async fn rebuild_app_prepare(
    app: &AppData,
) -> anyhow::Result<StateMachine<RebuildAppStates, Context>> {
    info!(
        "Rebuilding app {} at {}",
        app.name, &app.docker_compose_path
    );

    let mut sm = StateMachine::new(RebuildAppStates::RunDockerLogin, RebuildAppStates::Done);

    sm.add_handler(
        RebuildAppStates::RunDockerLogin,
        Arc::new(RunDockerLoginHandler::<RebuildAppStates> {
            next_state: RebuildAppStates::RunDockerComposePull,
            registry: app
                .settings
                .as_ref()
                .and_then(|settings| settings.registry.clone()),
        }),
    );
    sm.add_handler(
        RebuildAppStates::RunDockerComposePull,
        Arc::new(RunDockerComposeHandler::<RebuildAppStates> {
            next_state: RebuildAppStates::RunDockerComposeStop,
            command: ["build", "--pull"].iter().map(|s| s.to_string()).collect(),
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
    Ok(sm)
}

#[instrument(skip(app_state))]
pub async fn rebuild_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
    let sm = rebuild_app_prepare(app).await?;
    run_sm(app_state, app, sm).await
}
