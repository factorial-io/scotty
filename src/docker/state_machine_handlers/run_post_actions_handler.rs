use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::instrument;

use crate::{
    apps::app_data::AppSettings,
    settings::app_blueprint::{ActionName, AppBlueprint},
    state_machine::StateHandler,
};

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
    pub fn get_blueprint(&self, context: &Context) -> Option<AppBlueprint> {
        self.settings.as_ref()?;
        match &self.settings.as_ref().unwrap().app_blueprint {
            Some(blueprint_name) => {
                let blueprint = context
                    .app_state
                    .settings
                    .apps
                    .blueprints
                    .get(blueprint_name)?;
                Some(blueprint.clone())
            }
            None => None,
        }
    }

    pub fn get_environment(&self) -> std::collections::HashMap<String, String> {
        match &self.settings {
            Some(settings) => settings.environment.clone(),
            None => std::collections::HashMap::new(),
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
        let blueprint = self.get_blueprint(&context);
        if blueprint.is_none() {
            return Ok(self.next_state.clone());
        }

        let blueprint_actions = &blueprint.unwrap().actions;
        let selected_action = blueprint_actions.get(&self.action);

        if let Some(service_script_mapping) = selected_action {
            for (service, script) in service_script_mapping.iter() {
                let script_one_line = script.join("; ");
                let args = vec!["exec", service, "sh", "-c", &script_one_line];

                run_task_and_wait(
                    &context,
                    &docker_compose_path,
                    "docker-compose",
                    &args,
                    &self.get_environment(),
                    &format!("post-action {:?} on service {}", &self.action, service),
                )
                .await?;
            }
        }

        Ok(self.next_state.clone())
    }
}
