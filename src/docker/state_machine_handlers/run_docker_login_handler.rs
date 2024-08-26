use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, instrument};

use crate::{docker::docker_compose::run_task, state_machine::StateHandler};

use super::context::Context;

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
        debug!("Running docker login for {}", &registry.registry);
        let task_details = run_task(
            &context.app_state,
            &docker_compose_path,
            "docker",
            &args,
            context.task.clone(),
        )
        .await?;

        let handle = context
            .app_state
            .task_manager
            .get_task_handle(&task_details.id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        debug!("Waiting for docker login {} to finish", &registry.registry);
        while !handle.read().await.is_finished() {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let task = context
            .app_state
            .task_manager
            .get_task_details(&task_details.id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
        if let Some(last_exit_code) = task.last_exit_code {
            if last_exit_code != 0 {
                return Err(anyhow::anyhow!(
                    "Docker login failed with exit code {}",
                    last_exit_code
                ));
            }
        }
        debug!("docker login finished");

        Ok(self.next_state.clone())
    }
}
