#![allow(dead_code)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::apps::app_data::AppData;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema, Hash, Eq)]
pub struct GitlabContext {
    pub service_id: String,
    pub project_id: String,
    pub mr_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema, Hash, Eq)]
pub struct WebhookContext {
    pub service_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema, Hash, Eq)]
pub struct MattermostContext {
    pub service_id: String,
    pub channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema, Hash, Eq)]
pub enum NotificationReceiver {
    Log,
    Webhook(WebhookContext),
    Gitlab(GitlabContext),
    Mattermost(MattermostContext),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum MessageType {
    AppStarted,
    AppStopped,
    AppCreated,
    AppDestroyed,
    AppPurged,
    AppRebuilt,
    Custom(String),
}
impl MessageType {
    fn get_message(&self, app: &AppData) -> String {
        match &self {
            MessageType::AppStarted => format!("App {} started", app.name),
            MessageType::AppStopped => format!("App {} stopped", app.name),
            MessageType::AppCreated => format!("App {} created", app.name),
            MessageType::AppDestroyed => format!("App {} destroyed", app.name),
            MessageType::AppPurged => format!("App {} purged", app.name),
            MessageType::AppRebuilt => format!("App {} rebuilt", app.name),
            MessageType::Custom(msg) => msg.clone(),
        }
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub message_type: MessageType,
    pub app_name: String,
    pub message: String,
    pub urls: Vec<String>,
}

impl Message {
    pub fn new(message_type: MessageType, app: &AppData) -> Message {
        Message {
            message_type: message_type.clone(),
            app_name: app.name.clone(),
            message: message_type.get_message(app),
            urls: app.urls(),
        }
    }
}

#[async_trait]
pub trait NotificationImpl: Send {
    async fn notify(&self, msg: &Message) -> anyhow::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_creation() {
        let value = NotificationReceiver::Mattermost(MattermostContext {
            service_id: "mattermost".to_string(),
            channel: "test".to_string(),
        });
        let yaml_string = serde_yml::to_string(&value).expect("Failed to serialize to YAML");

        assert_eq!(
            yaml_string,
            "!Mattermost\nservice_id: mattermost\nchannel: test\n"
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AddNotificationRequest {
    pub app_name: String,
    pub service_ids: Vec<NotificationReceiver>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RemoveNotificationRequest {
    pub app_name: String,
    pub service_ids: Vec<NotificationReceiver>,
}
