use axum::async_trait;

use crate::app_state::AppState;

use crate::notification_types::{MattermostContext, Message, NotificationImpl};
use crate::settings::notification_services::MattermostSetting;

pub struct NotifyMattermost {
    settings: MattermostSetting,
    context: MattermostContext,
}

impl NotifyMattermost {
    pub fn new(state: &AppState, context: &MattermostContext) -> Self {
        let settings = state
            .settings
            .notification_services
            .get(&context.service_id);
        NotifyMattermost {
            settings: settings.unwrap().clone(),
            context: context.to_owned(),
        }
    }
}

#[async_trait]
impl NotificationImpl for NotifyMattermost {
    async fn notify(&self, msg: &Message) -> anyhow::Result<()> {
        Ok(())
    }
}
