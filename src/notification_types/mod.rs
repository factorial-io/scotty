use axum::async_trait;
use serde::{Deserialize, Serialize};

use crate::apps::app_data::AppData;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitlabContext {
    pub service_id: String,
    pub project_id: u32,
    pub mr_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MattermostContext {
    pub service_id: String,
    pub channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationReceiver {
    Log,
    Webhook,
    Gitlab(GitlabContext),
    Mattermost(MattermostContext),
}

#[derive(Debug, Clone)]
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
#[derive(Debug)]
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
