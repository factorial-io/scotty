use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{info, instrument};

use crate::{docker::find_apps::inspect_app, state_machine::StateHandler};

use super::context::Context;

#[derive(Debug)]
pub struct UpdateAppDataHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub next_state: S,
}

#[async_trait::async_trait]
impl<S> StateHandler<S, Context> for UpdateAppDataHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    #[instrument(skip(context))]
    async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
        let ctx = context.read().await;
        let docker_compose_path = std::path::PathBuf::from(&ctx.app_data.docker_compose_path);
        info!(
            "Updating app from docker-compose file {}",
            docker_compose_path.display(),
        );
        let app_data = inspect_app(&ctx.app_state, &docker_compose_path).await?;
        ctx.app_state.apps.update_app(app_data).await?;

        Ok(self.next_state.clone())
    }
}
