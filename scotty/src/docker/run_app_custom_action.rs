use std::sync::Arc;

use tracing::{info, instrument};

use crate::{
    api::error::AppError,
    app_state::SharedAppState,
    docker::{
        helper::run_sm,
        state_machine_handlers::{
            context::Context, run_docker_login_handler::RunDockerLoginHandler,
            run_post_actions_handler::RunPostActionsHandler,
            set_finished_handler::SetFinishedHandler,
            update_app_data_handler::UpdateAppDataHandler,
        },
    },
    state_machine::StateMachine,
};
use scotty_core::{
    notification_types::{Message, MessageType},
    settings::app_blueprint::ActionName,
    tasks::running_app_context::RunningAppContext,
};

use scotty_core::apps::app_data::{AppData, AppStatus};

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum RunAppCustomActionStates {
    RunDockerLogin,
    RunPostActions,
    UpdateAppData,
    SetFinished,
    Done,
}

#[instrument(skip(state))]
pub async fn run_app_custom_action_prepare(
    state: &SharedAppState,
    app: &AppData,
    action: ActionName,
) -> anyhow::Result<StateMachine<RunAppCustomActionStates, Context>> {
    if app.status != AppStatus::Running {
        return Err(AppError::AppNotRunning(app.name.to_string()).into());
    }

    // Check if the app has a blueprint with the custom action
    let app_settings = app.settings.clone();
    if app_settings.is_none() {
        return Err(AppError::AppSettingsNotFound(app.name.to_string()).into());
    }

    let blueprint_name = &app_settings.as_ref().unwrap().app_blueprint;
    if blueprint_name.is_none() {
        return Err(
            AppError::AppBlueprintMismatch("App doesn't have a blueprint".to_string()).into(),
        );
    }

    let blueprint = state
        .settings
        .apps
        .blueprints
        .get(blueprint_name.as_ref().unwrap());
    if blueprint.is_none() {
        return Err(
            AppError::AppBlueprintNotFound(blueprint_name.as_ref().unwrap().clone()).into(),
        );
    }

    // Verify the custom action exists in the blueprint
    if !blueprint.unwrap().actions.contains_key(&action) {
        return Err(
            AppError::ActionNotFound(format!("Action not found in blueprint: {action:?}")).into(),
        );
    }

    info!(
        app_name = %app.name,
        action = ?action,
        blueprint = ?blueprint_name.as_ref().unwrap(),
        "Starting custom action execution"
    );

    let mut sm = StateMachine::new(
        RunAppCustomActionStates::RunDockerLogin,
        RunAppCustomActionStates::Done,
    );

    sm.add_handler(
        RunAppCustomActionStates::RunDockerLogin,
        Arc::new(RunDockerLoginHandler::<RunAppCustomActionStates> {
            next_state: RunAppCustomActionStates::RunPostActions,
            registry: app.get_registry(),
        }),
    );

    sm.add_handler(
        RunAppCustomActionStates::RunPostActions,
        Arc::new(RunPostActionsHandler::<RunAppCustomActionStates> {
            next_state: RunAppCustomActionStates::UpdateAppData,
            action: action.clone(),
            settings: app.settings.clone(),
        }),
    );
    sm.add_handler(
        RunAppCustomActionStates::UpdateAppData,
        Arc::new(UpdateAppDataHandler::<RunAppCustomActionStates> {
            next_state: RunAppCustomActionStates::SetFinished,
        }),
    );
    sm.add_handler(
        RunAppCustomActionStates::SetFinished,
        Arc::new(SetFinishedHandler::<RunAppCustomActionStates> {
            next_state: RunAppCustomActionStates::Done,
            notification: Some(Message::new(
                MessageType::AppCustomActionCompleted(action.clone()),
                app,
            )),
        }),
    );

    Ok(sm)
}

#[instrument(skip(app_state))]
pub async fn run_app_custom_action(
    app_state: SharedAppState,
    app: &AppData,
    action: ActionName,
) -> anyhow::Result<RunningAppContext> {
    if app.status == AppStatus::Unsupported {
        return Err(AppError::OperationNotSupportedForLegacyApp(app.name.clone()).into());
    }

    let sm = run_app_custom_action_prepare(&app_state, app, action).await?;
    run_sm(app_state, app, sm).await
}
