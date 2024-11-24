use axum::async_trait;

use crate::notification_types::{Message, NotificationImpl};

pub struct NotifyLog;

impl NotifyLog {
    pub fn new() -> Self {
        NotifyLog {}
    }
}

#[async_trait]
impl NotificationImpl for NotifyLog {
    async fn notify(&self, msg: &Message) -> anyhow::Result<()> {
        println!("New notification: {:?}", msg);
        Ok(())
    }
}
