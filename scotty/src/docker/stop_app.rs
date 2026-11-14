use tracing::{info, instrument};

use crate::{api::error::AppError, app_state::SharedAppState};
use scotty_core::apps::app_data::{AppData, AppStatus};
use scotty_core::notification_types::{Message, MessageType};
use scotty_core::tasks::running_app_context::RunningAppContext;

use super::helper::{run_operation, OperationSpec};
use super::steps::compose::{docker_compose, update_app_data};
use super::steps::context::{Context, Step};

/// Intentionally no ensure/teardown of the app network here: `compose stop`
/// neither attaches nor validates networks, and the app's containers stay in
/// place (and on the per-app network), so the network must NOT be removed. Do
/// not add network teardown to stop: it would orphan running containers from
/// their network and break the next `app:run`.
pub async fn stop_steps(ctx: &Context) -> anyhow::Result<()> {
    docker_compose(ctx, &["stop"]).await?;
    update_app_data(ctx).await
}

async fn start_stop(
    app_state: SharedAppState,
    app: &AppData,
    operation: &str,
) -> anyhow::Result<RunningAppContext> {
    info!("Stopping app {} at {}", app.name, &app.docker_compose_path);
    run_operation(
        app_state,
        app,
        OperationSpec::exclusive(operation),
        &[Step::Stop],
        Some(Message::new(MessageType::AppStopped, app)),
        |ctx| async move { stop_steps(&ctx).await },
    )
    .await
}

#[instrument(skip(app_state))]
pub async fn stop_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
    if app.status == AppStatus::Unsupported {
        return Err(AppError::OperationNotSupportedForLegacyApp(app.name.clone()).into());
    }
    start_stop(app_state, app, "stop").await
}

#[instrument(skip(app_state))]
/// Stop for the TTL checker: also stops unsupported apps.
pub async fn force_stop_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
    start_stop(app_state, app, "ttl-stop").await
}
