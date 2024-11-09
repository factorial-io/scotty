#![allow(dead_code)]

use axum::async_trait;

use crate::{
    app_state::AppState,
    apps::app_data::{GitlabContext, NotificationReceiver},
};

#[async_trait]
pub trait NotificationImpl: Send {
    async fn notify(&self, msg: &str) -> anyhow::Result<()>;
}

struct NotifyLog;

impl NotifyLog {
    fn new() -> Self {
        NotifyLog {}
    }
}

#[async_trait]
impl NotificationImpl for NotifyLog {
    async fn notify(&self, msg: &str) -> anyhow::Result<()> {
        println!("New notification: {}", msg);
        Ok(())
    }
}

struct NotifyWebhook;
#[async_trait]
impl NotificationImpl for NotifyWebhook {
    async fn notify(&self, _msg: &str) -> anyhow::Result<()> {
        todo!();
    }
}

struct NotifyGitlab {
    context: GitlabContext,
}

impl NotifyGitlab {
    fn new(_state: &AppState, context: &GitlabContext) -> Self {
        // @todo
        NotifyGitlab {
            context: context.to_owned(),
        }
    }
}

#[async_trait]
impl NotificationImpl for NotifyGitlab {
    async fn notify(&self, _msg: &str) -> anyhow::Result<()> {
        todo!();
    }
}

async fn get_notification_receiver_impl(
    state: &AppState,
    to: &NotificationReceiver,
) -> anyhow::Result<Box<dyn NotificationImpl>> {
    match to {
        NotificationReceiver::Log => Ok(Box::new(NotifyLog::new())),
        NotificationReceiver::Gitlab(context) => Ok(Box::new(NotifyGitlab::new(state, context))),
        _ => Err(anyhow::anyhow!("Receiver not implemented")),
    }
}

pub async fn notify(
    app_state: &AppState,
    receivers: &[NotificationReceiver],
    msg: &str,
) -> anyhow::Result<()> {
    let results: Vec<anyhow::Result<()>> =
        futures_util::future::join_all(receivers.iter().map(|to| async {
            match get_notification_receiver_impl(app_state, to).await {
                Ok(helper) => helper.notify(msg).await,
                Err(err) => Err(err),
            }
        }))
        .await;

    for result in results {
        result?
    }
    Ok(())
}
