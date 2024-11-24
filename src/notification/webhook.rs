use axum::async_trait;

use crate::notification_types::{Message, NotificationImpl};

pub struct NotifyWebhook;

impl NotifyWebhook {
    pub fn new() -> Self {
        NotifyWebhook {}
    }
}
#[async_trait]
impl NotificationImpl for NotifyWebhook {
    async fn notify(&self, _msg: &Message) -> anyhow::Result<()> {
        todo!();
    }
}
