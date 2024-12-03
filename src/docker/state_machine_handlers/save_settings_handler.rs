use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::instrument;

use crate::{
    apps::app_data::{AppData, AppSettings},
    state_machine::StateHandler,
};

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
        let app = AppData {
            settings: Some(self.settings.clone()),
            ..context.app_data.clone()
        };
        app.save_settings().await?;

        Ok(self.next_state.clone())
    }
}
