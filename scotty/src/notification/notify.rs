#![allow(dead_code)]

use tracing::{error, instrument};

use crate::app_state::AppState;

use super::{
    gitlab::NotifyGitlab, log::NotifyLog, mattermost::NotifyMattermost, webhook::NotifyWebhook,
};
use scotty_core::notification_types::{Message, NotificationImpl, NotificationReceiver};

#[instrument(skip(state))]
async fn get_notification_receiver_impl(
    state: &AppState,
    to: &NotificationReceiver,
) -> anyhow::Result<Box<dyn NotificationImpl>> {
    let ns = &state.settings.notification_services;
    match to {
        NotificationReceiver::Log => Ok(Box::new(NotifyLog::new())),
        NotificationReceiver::Gitlab(context) => Ok(Box::new(NotifyGitlab::new(
            ns.get_gitlab(&context.service_id).ok_or(anyhow::anyhow!(
                "gitlab service {} not found in settings",
                context.service_id
            ))?,
            context,
        ))),
        NotificationReceiver::Webhook(context) => Ok(Box::new(NotifyWebhook::new(
            ns.get_webhook(&context.service_id).ok_or(anyhow::anyhow!(
                "webhook service {} not found in settings {:?}",
                context.service_id,
                state.settings.notification_services
            ))?,
            context,
        ))),
        NotificationReceiver::Mattermost(context) => Ok(Box::new(NotifyMattermost::new(
            ns.get_mattermost(&context.service_id)
                .ok_or(anyhow::anyhow!(
                    "mattermost service {} not found in settings {:?}",
                    context.service_id,
                    state.settings.notification_services
                ))?,
            context,
        ))),
    }
}

pub async fn notify<'a, I>(app_state: &AppState, receivers: I, msg: &Message) -> anyhow::Result<()>
where
    I: IntoIterator<Item = &'a NotificationReceiver>,
{
    let results: Vec<anyhow::Result<()>> =
        futures_util::future::join_all(receivers.into_iter().map(|to| async {
            match get_notification_receiver_impl(app_state, to).await {
                Ok(helper) => helper.notify(msg).await,
                Err(err) => Err(err),
            }
        }))
        .await;

    // We print errors
    for result in results {
        if let Err(err) = result {
            error!("Error notifying: {:?}", err);
        }
    }
    Ok(())
}
