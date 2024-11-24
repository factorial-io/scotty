use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::instrument;

use crate::{
    api::ws::broadcast_message, notification::notify::notify, notification_types::Message,
    state_machine::StateHandler, tasks::task_details::State,
};

use super::context::Context;

#[derive(Debug)]
pub struct SetFinishedHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub next_state: S,
    pub notification: Option<Message>,
}

#[async_trait::async_trait]
impl<S> StateHandler<S, Context> for SetFinishedHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    #[instrument(skip(context))]
    async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
        let context = context.read().await;
        let mut task_details = context.task.write().await;
        task_details.state = State::Finished;

        broadcast_message(
            &context.app_state,
            crate::api::message::WebSocketMessage::TaskInfoUpdated(task_details.clone()),
        )
        .await;

        if let (Some(app_settings), Some(notification)) =
            (&context.app_data.settings, &self.notification)
        {
            notify(&context.app_state, &app_settings.notify, notification).await?;
        }

        Ok(self.next_state.clone())
    }
}
