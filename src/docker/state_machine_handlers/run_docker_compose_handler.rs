use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, instrument};

use crate::{docker::docker_compose::run_docker_compose, state_machine::StateHandler};

use super::context::Context;

#[derive(Debug)]
pub struct RunDockerComposeHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub next_state: S,
    pub command: Vec<String>,
}

#[async_trait::async_trait]
impl<S> StateHandler<S, Context> for RunDockerComposeHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    #[instrument(skip(context))]
    async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
        let context = context.read().await;
        let docker_compose_path = std::path::PathBuf::from(&context.app_data.docker_compose_path);
        debug!("Running docker-compose {}", &self.command.join(" "));
        let task_details = run_docker_compose(
            &context.app_state,
            &docker_compose_path,
            self.command
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<&str>>()
                .as_slice(),
            context.task.clone(),
        )
        .await?;

        let handle = context
            .app_state
            .task_manager
            .get_task_handle(&task_details.id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        debug!(
            "Waiting for docker-compose {} to finish",
            &self.command.join(" ")
        );
        while !handle.read().await.is_finished() {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        debug!("docker-compose {} finished", &self.command.join(" "));

        Ok(self.next_state.clone())
    }
}
