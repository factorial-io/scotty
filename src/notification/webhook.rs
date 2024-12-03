#![allow(dead_code)]

use axum::async_trait;
use reqwest::Method;
use tracing::info;

use crate::notification_types::{Message, NotificationImpl, WebhookContext};
use crate::settings::notification_services::WebhookSettings;

pub struct NotifyWebhook {
    context: WebhookContext,
    settings: WebhookSettings,
}

impl NotifyWebhook {
    pub fn new(settings: &WebhookSettings, context: &WebhookContext) -> Self {
        NotifyWebhook {
            settings: settings.to_owned(),
            context: context.to_owned(),
        }
    }
}
impl NotifyWebhook {
    fn get_method(&self) -> Method {
        match self.settings.method.to_lowercase().as_str() {
            "get" => Method::GET,
            "post" => Method::POST,
            "put" => Method::PUT,
            "delete" => Method::DELETE,
            _ => Method::POST,
        }
    }
}

#[async_trait]
impl NotificationImpl for NotifyWebhook {
    async fn notify(&self, msg: &Message) -> anyhow::Result<()> {
        info!("Sending webhook to {}", self.settings.url);
        reqwest::Client::new()
            .request(self.get_method(), &self.settings.url)
            .json(msg)
            .send()
            .await?;
        Ok(())
    }
}
