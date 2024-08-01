use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{info, instrument};

use crate::{
    app_state::SharedAppState,
    apps::app_data::AppData,
    docker::state_machine_handlers::{
        Context, RunDockerComposeHandler, SetFinishedHandler, UpdateAppDataHandler,
    },
    state_machine::StateMachine,
    tasks::{running_app_context::RunningAppContext, task_details::TaskDetails},
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
    let context = Arc::new(RwLock::new(Context {
        app_state: app_state.clone(),
        app_data: app.clone(),
        task: Arc::new(RwLock::new(TaskDetails::default())),
    }));

    let _ = sm.spawn(context.clone());

    Ok(context.clone().read().await.as_running_app_context().await)
}
