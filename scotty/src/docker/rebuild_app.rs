use tracing::{info, instrument};

use crate::{api::error::AppError, app_state::SharedAppState};
use scotty_core::apps::app_data::{AppData, AppStatus};
use scotty_core::notification_types::{Message, MessageType};
use scotty_core::settings::app_blueprint::ActionName;
use scotty_core::tasks::running_app_context::RunningAppContext;

use super::helper::run_operation;
use super::steps::compose::{docker_compose, docker_login, update_app_data};
use super::steps::context::Context;
use super::steps::load_balancer::create_load_balancer_config;
use super::steps::network::ensure_app_network;
use super::steps::post_actions::run_post_actions;
use super::steps::wait_for_containers::wait_for_all_containers;

/// Rebuild and restart the app. Also used by create, which passes
/// `recreate_load_balancer_config = false` because it has just written the
/// load balancer config itself.
pub async fn rebuild_steps(
    ctx: &Context,
    recreate_load_balancer_config: bool,
) -> anyhow::Result<()> {
    if let (true, Some(settings)) = (recreate_load_balancer_config, &ctx.app_data.settings) {
        create_load_balancer_config(ctx, settings).await?;
    }
    docker_login(ctx).await?;
    // Ensure the per-app network exists before ANY compose subcommand runs:
    // the override declares it as external, and compose can reject commands
    // (e.g. on a freshly adopted app, or after the network was removed) when a
    // declared external network is missing.
    ensure_app_network(ctx).await?;
    docker_compose(ctx, &["pull"]).await?;
    docker_compose(ctx, &["build"]).await?;
    docker_compose(ctx, &["stop"]).await?;
    docker_compose(ctx, &["up", "-d"]).await?;
    wait_for_all_containers(ctx, Some(300)).await?;
    run_post_actions(ctx, &ActionName::PostRebuild).await?;
    update_app_data(ctx).await
}

#[instrument(skip(app_state))]
pub async fn rebuild_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
    if app.status == AppStatus::Unsupported {
        return Err(AppError::OperationNotSupportedForLegacyApp(app.name.clone()).into());
    }
    info!(
        "Rebuilding app {} at {}",
        app.name, &app.docker_compose_path
    );
    run_operation(
        app_state,
        app,
        Some(Message::new(MessageType::AppRebuilt, app)),
        |ctx| async move { rebuild_steps(&ctx, true).await },
    )
    .await
}
