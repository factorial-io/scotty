use std::sync::Arc;

use tracing::{info, instrument};

use crate::{
    api::error::AppError,
    app_state::SharedAppState,
    docker::state_machine_handlers::{
        context::Context, run_docker_compose_handler::RunDockerComposeHandler,
        run_docker_login_handler::RunDockerLoginHandler,
        run_post_actions_handler::RunPostActionsHandler,
        task_completion_handler::TaskCompletionHandler,
        update_app_data_handler::UpdateAppDataHandler,
        wait_for_all_containers_handler::WaitForAllContainersHandler,
    },
    state_machine::StateMachine,
};
use scotty_core::apps::app_data::{AppData, AppStatus};
use scotty_core::notification_types::{Message, MessageType};
use scotty_core::settings::app_blueprint::ActionName;
use scotty_core::tasks::running_app_context::RunningAppContext;

use super::helper::run_sm;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum RunAppStates {
    RunDockerLogin,
    RunDockerCompose,
    WaitForAllContainers,
    RunPostActions,
    UpdateAppData,
    SetFinished,
    SetFailed,
    Done,
}
#[instrument()]
async fn run_app_prepare(app: &AppData) -> anyhow::Result<StateMachine<RunAppStates, Context>> {
    info!("Running app {} at {}", app.name, &app.docker_compose_path);

    let mut sm = StateMachine::new(RunAppStates::RunDockerLogin, RunAppStates::Done);
    sm.set_error_state(RunAppStates::SetFailed);

    sm.add_handler(
        RunAppStates::RunDockerLogin,
        Arc::new(RunDockerLoginHandler::<RunAppStates> {
            next_state: RunAppStates::RunDockerCompose,
            registry: app.get_registry(),
        }),
    );

    sm.add_handler(
        RunAppStates::RunDockerCompose,
        Arc::new(RunDockerComposeHandler::<RunAppStates> {
            next_state: RunAppStates::WaitForAllContainers,
            command: ["up", "-d"].iter().map(|s| s.to_string()).collect(),
            env: app.get_environment(),
        }),
    );
    sm.add_handler(
        RunAppStates::WaitForAllContainers,
        Arc::new(WaitForAllContainersHandler::<RunAppStates> {
            next_state: RunAppStates::RunPostActions,
            timeout_seconds: Some(60),
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
        Arc::new(TaskCompletionHandler::success(
            RunAppStates::Done,
            Some(Message::new(MessageType::AppStarted, app)),
        )),
    );
    sm.add_handler(
        RunAppStates::SetFailed,
        Arc::new(TaskCompletionHandler::failure(RunAppStates::Done, None)),
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
    let result = run_sm(app_state.clone(), app, sm).await;

    match &result {
        Ok(_) => {
            info!("Successfully started app '{}'", app.name);
        }
        Err(e) => {
            info!("Failed to start app '{}': {}", app.name, e);
        }
    }

    result
}
