use std::{collections::HashMap, sync::Arc};

use scotty_core::{
    apps::app_data::AppSettings,
    settings::app_blueprint::{ActionName, AppBlueprint},
};
use tokio::sync::RwLock;
use tracing::instrument;

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
        let blueprint = self.get_blueprint(&context)?;
        if blueprint.is_none() {
            return Ok(self.next_state.clone());
        }

        let blueprint_actions = &blueprint.unwrap().actions;
        let selected_action = blueprint_actions.get(&self.action);

        let environment = context.app_data.get_environment();
        let augmented_env = context.app_data.augment_environment(HashMap::new());

        if let Some(service_script_mapping) = selected_action {
            for (service, script) in service_script_mapping.iter() {
                let mut augmented_script = Vec::new();
                for (key, value) in augmented_env.iter() {
                    augmented_script.push(format!("export {}={}", key, value));
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
                    &format!("post-action {:?} on service {}", &self.action, service),
                )
                .await?;
            }
        }

        Ok(self.next_state.clone())
    }
}
