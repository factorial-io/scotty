use axum::async_trait;

use crate::app_state::AppState;

use crate::notification_types::{GitlabContext, Message, NotificationImpl};

pub struct NotifyGitlab {
    context: GitlabContext,
}

impl NotifyGitlab {
    pub fn new(_state: &AppState, context: &GitlabContext) -> Self {
        // @todo
        NotifyGitlab {
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
