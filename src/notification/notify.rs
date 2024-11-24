#![allow(dead_code)]

use crate::app_state::AppState;

use super::{
    gitlab::NotifyGitlab, log::NotifyLog, mattermost::NotifyMattermost, webhook::NotifyWebhook,
};
use crate::notification_types::{Message, NotificationImpl, NotificationReceiver};

async fn get_notification_receiver_impl(
    state: &AppState,
    to: &NotificationReceiver,
) -> anyhow::Result<Box<dyn NotificationImpl>> {
    match to {
        NotificationReceiver::Log => Ok(Box::new(NotifyLog::new())),
        NotificationReceiver::Gitlab(context) => Ok(Box::new(NotifyGitlab::new(state, context))),
        NotificationReceiver::Webhook => Ok(Box::new(NotifyWebhook::new())),
        NotificationReceiver::Mattermost(context) => {
            Ok(Box::new(NotifyMattermost::new(state, context)))
        }
    }
}

pub async fn notify(
    app_state: &AppState,
    receivers: &[NotificationReceiver],
    msg: &Message,
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
