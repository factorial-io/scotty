use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{info, instrument};

use crate::{apps::app_data::AppSettings, state_machine::StateHandler};

use super::context::Context;

#[derive(Debug)]
pub struct SaveSettingsHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub settings: AppSettings,
    pub next_state: S,
}

#[async_trait::async_trait]
impl<S> StateHandler<S, Context> for SaveSettingsHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    #[instrument(skip(context))]
    async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
        let context = context.read().await;
        let root_directory = std::path::PathBuf::from(&context.app_data.root_directory);

        let settings_path = root_directory.join(".scotty.yml");
        info!("Saving settings to {}", settings_path.display());
        let settings_yaml = serde_yml::to_string(&self.settings)?;
        tokio::fs::write(&settings_path, settings_yaml).await?;

        Ok(self.next_state.clone())
    }
}
