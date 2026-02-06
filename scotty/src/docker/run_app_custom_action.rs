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
            task_completion_handler::TaskCompletionHandler,
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
pub(crate) enum RunAppCustomActionStates {
    RunDockerLogin,
    RunPostActions,
    UpdateAppData,
    SetFinished,
    SetFailed,
    Done,
}

/// Check if the action exists - either as a per-app custom action or in the blueprint.
/// Returns Ok(()) if the action is found, Err otherwise.
fn validate_action_exists(
    state: &SharedAppState,
    app: &AppData,
    action: &ActionName,
) -> Result<(), AppError> {
    let app_settings = app
        .settings
        .as_ref()
        .ok_or_else(|| AppError::AppSettingsNotFound(app.name.to_string()))?;

    // Extract the action name string from ActionName enum
    let action_name_str = match action {
        ActionName::Custom(name) => name.as_str(),
        _ => {
            // For built-in actions, check blueprint
            return validate_blueprint_action(state, app_settings, action);
        }
    };

    // First, check per-app custom actions
    if app_settings.get_custom_action(action_name_str).is_some() {
        return Ok(());
    }

    // Fall back to blueprint actions
    validate_blueprint_action(state, app_settings, action)
}

/// Validate that an action exists in the blueprint
fn validate_blueprint_action(
    state: &SharedAppState,
    app_settings: &scotty_core::apps::app_data::AppSettings,
    action: &ActionName,
) -> Result<(), AppError> {
    let blueprint_name = app_settings.app_blueprint.as_ref().ok_or_else(|| {
        AppError::ActionNotFound(format!(
            "Action {:?} not found: app has no custom actions and no blueprint",
            action
        ))
    })?;

    let blueprint = state
        .settings
        .apps
        .blueprints
        .get(blueprint_name)
        .ok_or_else(|| AppError::AppBlueprintNotFound(blueprint_name.clone()))?;

    if !blueprint.actions.contains_key(action) {
        return Err(AppError::ActionNotFound(format!(
            "Action {:?} not found in app custom actions or blueprint '{}'",
            action, blueprint_name
        )));
    }

    Ok(())
}

#[allow(private_interfaces)]
#[instrument(skip(state))]
pub async fn run_app_custom_action_prepare(
    state: &SharedAppState,
    app: &AppData,
    action: ActionName,
) -> anyhow::Result<StateMachine<RunAppCustomActionStates, Context>> {
    if app.status != AppStatus::Running {
        return Err(AppError::AppNotRunning(app.name.to_string()).into());
    }

    // Validate that the action exists (either per-app or in blueprint)
    validate_action_exists(state, app, &action)?;

    info!(
        app_name = %app.name,
        action = ?action,
        "Starting custom action execution"
    );

    let mut sm = StateMachine::new(
        RunAppCustomActionStates::RunDockerLogin,
        RunAppCustomActionStates::Done,
    );
    sm.set_error_state(RunAppCustomActionStates::SetFailed);

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
        Arc::new(TaskCompletionHandler::success(
            RunAppCustomActionStates::Done,
            Some(Message::new(
                MessageType::AppCustomActionCompleted(action.clone()),
                app,
            )),
        )),
    );
    sm.add_handler(
        RunAppCustomActionStates::SetFailed,
        Arc::new(TaskCompletionHandler::failure(
            RunAppCustomActionStates::Done,
            None,
        )),
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
