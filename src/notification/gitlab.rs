#![allow(dead_code)]

use axum::async_trait;

use crate::notification_types::{GitlabContext, Message, NotificationImpl};
use crate::settings::notification_services::GitlabSettings;

pub struct NotifyGitlab {
    context: GitlabContext,
    settings: GitlabSettings,
}

impl NotifyGitlab {
    pub fn new(settings: &GitlabSettings, context: &GitlabContext) -> Self {
        // @todo
        NotifyGitlab {
            settings: settings.to_owned(),
            context: context.to_owned(),
        }
    }
}

#[async_trait]
impl NotificationImpl for NotifyGitlab {
    async fn notify(&self, _msg: &Message) -> anyhow::Result<()> {
        todo!();
    }
}
