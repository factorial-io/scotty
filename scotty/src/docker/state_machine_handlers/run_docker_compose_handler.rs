use std::sync::Arc;

use scotty_core::utils::secret::SecretHashMap;
use tokio::sync::RwLock;
use tracing::instrument;

use crate::state_machine::StateHandler;

use super::{context::Context, run_task_and_wait::run_task_and_wait};

#[derive(Debug)]
pub struct RunDockerComposeHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub next_state: S,
    pub command: Vec<String>,
    pub env: SecretHashMap,
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
        run_task_and_wait(
            &context,
            &docker_compose_path,
            "docker-compose",
            self.command
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<&str>>()
                .as_slice(),
            &self.env,
            &format!("docker-compose {}", &self.command.join(" ")),
        )
        .await?;

        Ok(self.next_state.clone())
    }
}
