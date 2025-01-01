use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{info, instrument};

use crate::state_machine::StateHandler;

use super::context::Context;

#[derive(Debug)]
pub struct CreateDirectoryHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub next_state: S,
}

#[async_trait::async_trait]
impl<S> StateHandler<S, Context> for CreateDirectoryHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    #[instrument(skip(context))]
    async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
        let context = context.read().await;
        let root_directory = std::path::PathBuf::from(&context.app_data.root_directory);

        info!("Creating directory {}", root_directory.display());
        if !root_directory.exists() {
            std::fs::create_dir_all(root_directory)?;
        }

        Ok(self.next_state.clone())
    }
}
