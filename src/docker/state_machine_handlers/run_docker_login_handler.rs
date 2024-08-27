use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::instrument;

use crate::state_machine::StateHandler;

use super::{context::Context, run_task_and_wait::run_task_and_wait};

#[derive(Debug)]
pub struct RunDockerLoginHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub next_state: S,
    pub registry: Option<String>,
}

#[async_trait::async_trait]
impl<S> StateHandler<S, Context> for RunDockerLoginHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    #[instrument(skip(context))]
    async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
        let context = context.read().await;
        let docker_compose_path = std::path::PathBuf::from(&context.app_data.docker_compose_path);

        // Bail out early
        if self.registry.is_none() {
            return Ok(self.next_state.clone());
        }

        let registry = self.registry.as_ref().unwrap();
        let registry = context
            .app_state
            .settings
            .docker
            .registries
            .get(registry)
            .ok_or_else(|| anyhow::anyhow!("Registry {} not found in settings!", registry))?;
        let args = vec![
            "login",
            &registry.registry,
            "-u",
            &registry.username,
            "-p",
            &registry.password,
        ];

        run_task_and_wait(
            &context,
            &docker_compose_path,
            "docker",
            &args,
            &format!("Log into registry {}", &registry.registry),
        )
        .await?;

        Ok(self.next_state.clone())
    }
}
