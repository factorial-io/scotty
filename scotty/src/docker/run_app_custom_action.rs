use tracing::{info, instrument};

use crate::{
    api::error::AppError,
    app_state::SharedAppState,
    docker::{
        helper::run_operation,
        steps::{
            compose::{docker_login, update_app_data},
            context::Context,
            post_actions::run_post_actions,
        },
    },
};
use scotty_core::{
    notification_types::{Message, MessageType},
    settings::app_blueprint::ActionName,
    tasks::running_app_context::RunningAppContext,
};

use scotty_core::apps::app_data::{AppData, AppStatus};

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

pub async fn custom_action_steps(ctx: &Context, action: &ActionName) -> anyhow::Result<()> {
    docker_login(ctx).await?;
    run_post_actions(ctx, action).await?;
    update_app_data(ctx).await
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
    if app.status != AppStatus::Running {
        return Err(AppError::AppNotRunning(app.name.to_string()).into());
    }
    validate_action_exists(&app_state, app, &action)?;

    info!(
        app_name = %app.name,
        action = ?action,
        "Starting custom action execution"
    );
    let notification = Message::new(MessageType::AppCustomActionCompleted(action.clone()), app);
    run_operation(app_state, app, Some(notification), move |ctx| async move {
        custom_action_steps(&ctx, &action).await
    })
    .await
}
