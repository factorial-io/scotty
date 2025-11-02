use async_trait::async_trait;

use scotty_core::notification_types::{Message, NotificationImpl};

pub struct NotifyLog;

impl Default for NotifyLog {
    fn default() -> Self {
        Self::new()
    }
}

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
