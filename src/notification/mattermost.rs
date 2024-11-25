use axum::async_trait;
use serde::Serialize;

use crate::notification_types::{MattermostContext, Message, NotificationImpl};
use crate::settings::notification_services::MattermostSettings;

pub struct NotifyMattermost {
    settings: MattermostSettings,
    context: MattermostContext,
}

impl NotifyMattermost {
    pub fn new(settings: &MattermostSettings, context: &MattermostContext) -> Self {
        NotifyMattermost {
            settings: settings.to_owned(),
            context: context.to_owned(),
        }
    }
}

#[derive(Serialize)]
struct MattermostMessage {
    channel: String,
    username: String,
    text: String,
}

#[async_trait]
impl NotificationImpl for NotifyMattermost {
    async fn notify(&self, msg: &Message) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/hooks/{}", self.settings.host, self.settings.hook_id);

        let payload = MattermostMessage {
            channel: self.context.channel.clone(),
            username: "scotty".to_string(),
            text: format!("{}\n\n* {}", msg.message, msg.urls.join("\n* ")),
        };

        // Serialize the message
        let message_body = serde_json::to_string(&payload)?;

        // Send the message to Mattermost using an HTTP POST request
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(message_body)
            .send()
            .await?;

        // Check if the response indicates success
        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Failed to send message to Mattermost: {:?}",
                response.status()
            ))
        }
    }
}
