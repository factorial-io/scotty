use std::collections::BTreeMap;

use anyhow::Context as _;
use scotty_core::{
    apps::app_data::AppData,
    settings::app_blueprint::{ActionName, AppBlueprint},
    utils::secret::SecretHashMap,
};
use tracing::{info, instrument};

use crate::api::error::AppError;
use crate::app_state::SharedAppState;
use crate::tasks::manager::OutputMode;

use super::{
    context::{Context, Step},
    run_task_and_wait::run_task_and_wait,
};

/// Commands to execute for an action (service -> list of commands), by
/// service name so they run in a stable order.
type ActionCommands = BTreeMap<String, Vec<String>>;

fn get_blueprint(
    app_state: &SharedAppState,
    app: &AppData,
) -> Result<Option<AppBlueprint>, AppError> {
    let Some(settings) = &app.settings else {
        return Ok(None);
    };
    match &settings.app_blueprint {
        Some(blueprint_name) => {
            let blueprint = app_state.settings.apps.blueprints.get(blueprint_name);

            if blueprint.is_none() {
                return Err(AppError::AppBlueprintMismatch(blueprint_name.to_string()));
            }
            Ok(Some(blueprint.unwrap().clone()))
        }
        None => Ok(None),
    }
}

/// Get the commands for the action, checking per-app custom actions first, then blueprint.
/// Returns None if the action is not found in either location.
fn get_action_commands(
    app_state: &SharedAppState,
    app: &AppData,
    action: &ActionName,
) -> Result<Option<ActionCommands>, AppError> {
    let settings = match &app.settings {
        Some(s) => s,
        None => return Ok(None),
    };

    // Extract action name string for per-app custom action lookup
    let action_name_str = match action {
        ActionName::Custom(name) => Some(name.as_str()),
        _ => None,
    };

    // First, check per-app custom actions
    if let Some(name) = action_name_str {
        if let Some(custom_action) = settings.get_custom_action(name) {
            info!(
                action = %name,
                source = "per-app custom action",
                "Found action in per-app custom actions"
            );
            return Ok(Some(custom_action.commands.clone().into_iter().collect()));
        }
    }

    // Fall back to blueprint actions
    let blueprint = get_blueprint(app_state, app)?;
    if let Some(bp) = blueprint {
        if let Some(blueprint_action) = bp.actions.get(action) {
            info!(
                action = ?action,
                source = "blueprint",
                "Found action in blueprint"
            );
            return Ok(Some(
                blueprint_action.commands.clone().into_iter().collect(),
            ));
        }
    }

    Ok(None)
}

/// The steps of running `action` on `app`: one per service command.
pub fn action_steps(
    app_state: &SharedAppState,
    app: &AppData,
    action: &ActionName,
) -> Result<Vec<Step>, AppError> {
    let name = String::from(action.clone());
    Ok(get_action_commands(app_state, app, action)?
        .unwrap_or_default()
        .keys()
        .map(|service| Step::command(&name, service))
        .collect())
}

/// Run the app's commands for `action` (per-app custom action or blueprint
/// action) on each service. A no-op when the app defines none.
#[instrument(skip_all, fields(app = %context.app_data.name, action = ?action))]
pub async fn run_post_actions(context: &Context, action: &ActionName) -> anyhow::Result<()> {
    // Get action commands from either per-app custom actions or blueprint
    let Some(commands) = get_action_commands(&context.app_state, &context.app_data, action)? else {
        info!(
            app_name = %context.app_data.name,
            action = ?action,
            "No action commands found, skipping"
        );
        context.skip_step(&Step::PostActions).await;
        return Ok(());
    };
    context
        .step(&Step::PostActions, run_commands(context, action, commands))
        .await
}

async fn run_commands(
    context: &Context,
    action: &ActionName,
    commands: ActionCommands,
) -> anyhow::Result<()> {
    {
        let docker_compose_path = std::path::PathBuf::from(&context.app_data.docker_compose_path);
        let action_name = String::from(action.clone());

        // Two different environment variables are used for different purposes:
        // - `environment`: Full environment (app settings + augmented SCOTTY__* vars)
        //   This is passed to the docker-compose command itself for variable substitution in docker-compose.yml
        // - `augmented_env`: Only the augmented SCOTTY__* variables (APP_NAME, PUBLIC_URL__*, etc.)
        //   These are explicitly exported in the shell script for convenience, so custom action scripts
        //   can easily access Scotty-provided metadata without relying on docker-compose.yml environment section
        // The context's app_data is a snapshot from before `compose up` on
        // create; the app list holds the inspected one with services and URLs.
        let app_data = context
            .app_state
            .apps
            .get_app(&context.app_data.name)
            .await
            .unwrap_or_else(|| context.app_data.clone());
        let environment = app_data.get_environment();
        let augmented_env = app_data.augment_environment(SecretHashMap::new());

        info!(
            app_name = %context.app_data.name,
            action = ?action,
            service_count = commands.len(),
            "Executing custom action on {} service(s)",
            commands.len()
        );

        for (service, script) in &commands {
            info!(
                app_name = %context.app_data.name,
                action = ?action,
                service = %service,
                script_lines = script.len(),
                "Running action on service"
            );

            let mut augmented_script = Vec::new();
            for (key, value) in augmented_env.iter() {
                // Use expose_secret() to get the real value, not the masked display value
                augmented_script.push(format!("export {key}={}", value.expose_secret()));
            }
            augmented_script.extend(script.iter().cloned());
            let script_one_line = augmented_script.join("; ");
            let args = vec!["exec", service, "sh", "-c", &script_one_line];

            let msg = format!("action {:?} on service {}", action, service);
            let run = run_task_and_wait(
                context,
                &docker_compose_path,
                "docker-compose",
                &args,
                &environment,
                &msg,
                OutputMode::Plain,
            );
            context
                .step(&Step::command(&action_name, service), run)
                .await
                .context(msg)?;
        }

        Ok(())
    }
}
