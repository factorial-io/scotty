use std::{collections::HashMap, sync::Arc};

use scotty_core::{
    apps::app_data::AppSettings,
    settings::app_blueprint::{ActionName, AppBlueprint},
    utils::secret::SecretHashMap,
};
use tokio::sync::RwLock;
use tracing::{info, instrument};

use crate::{api::error::AppError, state_machine::StateHandler};

use super::{context::Context, run_task_and_wait::run_task_and_wait};

#[derive(Debug)]
pub struct RunPostActionsHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub next_state: S,
    pub action: ActionName,
    pub settings: Option<AppSettings>,
}

/// Commands to execute for an action (service -> list of commands)
type ActionCommands = HashMap<String, Vec<String>>;

impl<S> RunPostActionsHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub fn get_blueprint(&self, context: &Context) -> Result<Option<AppBlueprint>, AppError> {
        if self.settings.as_ref().is_none() {
            return Ok(None);
        }
        match &self.settings.as_ref().unwrap().app_blueprint {
            Some(blueprint_name) => {
                let blueprint = context
                    .app_state
                    .settings
                    .apps
                    .blueprints
                    .get(blueprint_name);

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
    fn get_action_commands(&self, context: &Context) -> Result<Option<ActionCommands>, AppError> {
        let settings = match &self.settings {
            Some(s) => s,
            None => return Ok(None),
        };

        // Extract action name string for per-app custom action lookup
        let action_name_str = match &self.action {
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
                return Ok(Some(custom_action.commands.clone()));
            }
        }

        // Fall back to blueprint actions
        let blueprint = self.get_blueprint(context)?;
        if let Some(bp) = blueprint {
            if let Some(blueprint_action) = bp.actions.get(&self.action) {
                info!(
                    action = ?self.action,
                    source = "blueprint",
                    "Found action in blueprint"
                );
                return Ok(Some(blueprint_action.commands.clone()));
            }
        }

        Ok(None)
    }
}

#[async_trait::async_trait]
impl<S> StateHandler<S, Context> for RunPostActionsHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    #[instrument(skip(context))]
    async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
        let context = context.read().await;
        let docker_compose_path = std::path::PathBuf::from(&context.app_data.docker_compose_path);

        // Get action commands from either per-app custom actions or blueprint
        let action_commands = self.get_action_commands(&context)?;
        if action_commands.is_none() {
            info!(
                app_name = %context.app_data.name,
                action = ?self.action,
                "No action commands found, skipping"
            );
            return Ok(self.next_state.clone());
        }

        let commands = action_commands.unwrap();

        // Two different environment variables are used for different purposes:
        // - `environment`: Full environment (app settings + augmented SCOTTY__* vars)
        //   This is passed to the docker-compose command itself for variable substitution in docker-compose.yml
        // - `augmented_env`: Only the augmented SCOTTY__* variables (APP_NAME, PUBLIC_URL__*, etc.)
        //   These are explicitly exported in the shell script for convenience, so custom action scripts
        //   can easily access Scotty-provided metadata without relying on docker-compose.yml environment section
        let environment = context.app_data.get_environment();
        let augmented_env = context.app_data.augment_environment(SecretHashMap::new());

        info!(
            app_name = %context.app_data.name,
            action = ?self.action,
            service_count = commands.len(),
            "Executing custom action on {} service(s)",
            commands.len()
        );

        for (service, script) in &commands {
            info!(
                app_name = %context.app_data.name,
                action = ?self.action,
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

            run_task_and_wait(
                &context,
                &docker_compose_path,
                "docker-compose",
                &args,
                &environment,
                &format!("action {:?} on service {}", &self.action, service),
            )
            .await?;
        }

        Ok(self.next_state.clone())
    }
}
