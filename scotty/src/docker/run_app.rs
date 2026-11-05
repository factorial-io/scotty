use tracing::{info, instrument};

use crate::api::error::AppError;
use crate::app_state::{SharedAppState, WakeGuard};
use scotty_core::apps::app_data::{AppData, AppStatus};
use scotty_core::notification_types::{Message, MessageType};
use scotty_core::settings::app_blueprint::ActionName;
use scotty_core::tasks::running_app_context::RunningAppContext;

use super::helper::run_operation;
use super::steps::compose::{docker_compose, docker_login, update_app_data};
use super::steps::context::Context;
use super::steps::network::ensure_app_network;
use super::steps::post_actions::run_post_actions;
use super::steps::wait_for_containers::wait_for_all_containers;

pub async fn run_steps(ctx: &Context) -> anyhow::Result<()> {
    docker_login(ctx).await?;
    ensure_app_network(ctx).await?;
    docker_compose(ctx, &["up", "-d"]).await?;
    wait_for_all_containers(ctx, Some(60)).await?;
    run_post_actions(ctx, &ActionName::PostRun).await?;
    update_app_data(ctx).await
}

#[instrument(skip(app_state))]
pub async fn run_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
    if app.status == AppStatus::Unsupported {
        return Err(AppError::OperationNotSupportedForLegacyApp(app.name.clone()).into());
    }
    info!("Running app {} at {}", app.name, &app.docker_compose_path);
    run_operation(
        app_state,
        app,
        Some(Message::new(MessageType::AppStarted, app)),
        |ctx| async move { run_steps(&ctx).await },
    )
    .await
}

/// Start an app on behalf of an incoming request (autowake): same steps as
/// [`run_app`], but without the `AppStarted` notification. `guard` is held
/// until the steps finish, so concurrent requests don't start the app twice.
#[instrument(skip(app_state, guard))]
pub async fn wake_app(
    app_state: SharedAppState,
    app: &AppData,
    guard: WakeGuard,
) -> anyhow::Result<RunningAppContext> {
    if app.status == AppStatus::Unsupported {
        return Err(AppError::OperationNotSupportedForLegacyApp(app.name.clone()).into());
    }
    info!("Waking app {} at {}", app.name, &app.docker_compose_path);
    run_operation(app_state, app, None, |ctx| async move {
        let _guard = guard;
        run_steps(&ctx).await
    })
    .await
}
