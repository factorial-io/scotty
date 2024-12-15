use std::sync::Arc;

use tracing::{info, instrument};

use crate::{
    api::error::AppError,
    app_state::SharedAppState,
    apps::app_data::{AppData, AppStatus},
    docker::state_machine_handlers::{
        context::Context, create_load_balancer_config::CreateLoadBalancerConfig,
        run_docker_compose_handler::RunDockerComposeHandler,
        run_docker_login_handler::RunDockerLoginHandler,
        run_post_actions_handler::RunPostActionsHandler, set_finished_handler::SetFinishedHandler,
        update_app_data_handler::UpdateAppDataHandler,
    },
    notification_types::{Message, MessageType},
    settings::app_blueprint::ActionName,
    state_machine::StateMachine,
    tasks::running_app_context::RunningAppContext,
};

use super::helper::run_sm;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum RebuildAppStates {
    RecreateLoadBalancerConfig,
    RunDockerLogin,
    RunDockerComposePull,
    RunDockerComposeBuild,
    RunDockerComposeStop,
    RunDockerComposeRun,
    RunPostActions,
    UpdateAppData,
    SetFinished,
    Done,
}

#[instrument]
pub async fn rebuild_app_prepare(
    app_state: &SharedAppState,
    app: &AppData,
    recreate_load_balancer_config: bool,
) -> anyhow::Result<StateMachine<RebuildAppStates, Context>> {
    info!(
        "Rebuilding app {} at {}",
        app.name, &app.docker_compose_path
    );

    let start_with_recreate = app.settings.is_some() && recreate_load_balancer_config;

    let mut sm = StateMachine::new(
        match start_with_recreate {
            true => RebuildAppStates::RecreateLoadBalancerConfig,
            false => RebuildAppStates::RunDockerLogin,
        },
        RebuildAppStates::Done,
    );

    if start_with_recreate {
        sm.add_handler(
            RebuildAppStates::RecreateLoadBalancerConfig,
            Arc::new(CreateLoadBalancerConfig::<RebuildAppStates> {
                next_state: RebuildAppStates::RunDockerLogin,
                load_balancer_type: app_state.settings.load_balancer_type.clone(),
                settings: app.settings.as_ref().unwrap().clone(),
            }),
        );
    }
    sm.add_handler(
        RebuildAppStates::RunDockerLogin,
        Arc::new(RunDockerLoginHandler::<RebuildAppStates> {
            next_state: RebuildAppStates::RunDockerComposePull,
            registry: app.get_registry(),
        }),
    );
    sm.add_handler(
        RebuildAppStates::RunDockerComposePull,
        Arc::new(RunDockerComposeHandler::<RebuildAppStates> {
            next_state: RebuildAppStates::RunDockerComposeBuild,
            command: ["pull"].iter().map(|s| s.to_string()).collect(),
            env: app.get_environment(),
        }),
    );
    sm.add_handler(
        RebuildAppStates::RunDockerComposeBuild,
        Arc::new(RunDockerComposeHandler::<RebuildAppStates> {
            next_state: RebuildAppStates::RunDockerComposeStop,
            command: ["build"].iter().map(|s| s.to_string()).collect(),
            env: app.get_environment(),
        }),
    );
    sm.add_handler(
        RebuildAppStates::RunDockerComposeStop,
        Arc::new(RunDockerComposeHandler::<RebuildAppStates> {
            next_state: RebuildAppStates::RunDockerComposeRun,
            command: ["stop"].iter().map(|s| s.to_string()).collect(),
            env: app.get_environment(),
        }),
    );
    sm.add_handler(
        RebuildAppStates::RunDockerComposeRun,
        Arc::new(RunDockerComposeHandler::<RebuildAppStates> {
            next_state: RebuildAppStates::RunPostActions,
            command: ["up", "-d"].iter().map(|s| s.to_string()).collect(),
            env: app.get_environment(),
        }),
    );
    sm.add_handler(
        RebuildAppStates::RunPostActions,
        Arc::new(RunPostActionsHandler::<RebuildAppStates> {
            next_state: RebuildAppStates::UpdateAppData,
            action: ActionName::PostRebuild,
            settings: app.settings.clone(),
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
            notification: Some(Message::new(MessageType::AppRebuilt, app)),
        }),
    );
    Ok(sm)
}

#[instrument(skip(app_state))]
pub async fn rebuild_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
    if app.status == AppStatus::Unsupported {
        return Err(AppError::OperationNotSupportedForLegacyApp(app.name.clone()).into());
    }
    let sm = rebuild_app_prepare(&app_state, app, true).await?;
    run_sm(app_state, app, sm).await
}
