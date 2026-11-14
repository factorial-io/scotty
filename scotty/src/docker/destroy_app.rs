use anyhow::Context as _;

use crate::api::error::AppError;
use crate::app_state::SharedAppState;
use scotty_core::apps::app_data::AppData;
use scotty_core::apps::app_data::AppStatus;
use scotty_core::notification_types::Message;
use scotty_core::notification_types::MessageType;
use scotty_core::tasks::running_app_context::RunningAppContext;

use super::helper::{run_operation, OperationSpec};
use super::purge_app::{purge_steps, PurgeAppMethod, PURGE_STEPS};
use super::steps::context::{Context, Step};
use super::steps::files::remove_directory;

pub async fn destroy_steps(ctx: &Context) -> anyhow::Result<()> {
    // Compose down, proxy network teardown and the app data refresh are the
    // purge steps; that is why destroy has no network teardown of its own.
    purge_steps(ctx, PurgeAppMethod::Down)
        .await
        .context("Docker compose down failed")?;
    ctx.step(&Step::RemoveDirectory, remove_directory(ctx))
        .await?;
    ctx.app_state.apps.remove_app(&ctx.app_data.name).await?;
    Ok(())
}

/// The lock spec of a destroy named `operation`: removing the app changes the
/// app list.
pub(crate) fn operation(operation: &str) -> OperationSpec {
    OperationSpec::exclusive(operation).changing_app_list()
}

/// Destroy `app`; `operation` names it in the app's lock (`destroy`, or
/// `ttl-destroy` for the TTL checker).
pub async fn destroy_app(
    app_state: SharedAppState,
    app: &AppData,
    operation: &str,
) -> anyhow::Result<RunningAppContext> {
    if app.status == AppStatus::Unsupported {
        return Err(AppError::OperationNotSupportedForLegacyApp(app.name.clone()).into());
    }
    run_operation(
        app_state,
        app,
        self::operation(operation),
        &[PURGE_STEPS.as_slice(), &[Step::RemoveDirectory]].concat(),
        Some(Message::new(MessageType::AppDestroyed, app)),
        |ctx| async move { destroy_steps(&ctx).await },
    )
    .await
}
