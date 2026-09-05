use tracing::{info, instrument};

use crate::{api::error::AppError, app_state::SharedAppState};
use scotty_core::apps::app_data::{AppData, AppStatus};
use scotty_core::notification_types::{Message, MessageType};
use scotty_core::tasks::running_app_context::RunningAppContext;

use super::helper::run_operation;
use super::steps::compose::{docker_compose, update_app_data};
use super::steps::context::Context;
use super::steps::network::teardown_app_network;

#[derive(Copy, Clone, Debug)]
pub enum PurgeAppMethod {
    Down,
    Rm,
}

impl PurgeAppMethod {
    pub fn compose_args(self) -> &'static [&'static str] {
        match self {
            PurgeAppMethod::Down => &["down", "-v", "--rmi", "all"],
            PurgeAppMethod::Rm => &["rm", "-s", "-f"],
        }
    }
}

/// Remove the app's containers and its proxy network. Also used by destroy.
pub async fn purge_steps(ctx: &Context, method: PurgeAppMethod) -> anyhow::Result<()> {
    docker_compose(ctx, method.compose_args()).await?;
    teardown_app_network(ctx).await?;
    update_app_data(ctx).await
}

#[instrument(skip(app_state))]
pub async fn purge_app(
    app_state: SharedAppState,
    app: &AppData,
) -> anyhow::Result<RunningAppContext> {
    if app.status == AppStatus::Unsupported {
        return Err(AppError::OperationNotSupportedForLegacyApp(app.name.clone()).into());
    }
    info!("Purging app {} at {}", app.name, &app.docker_compose_path);
    run_operation(
        app_state,
        app,
        Some(Message::new(MessageType::AppPurged, app)),
        |ctx| async move { purge_steps(&ctx, PurgeAppMethod::Rm).await },
    )
    .await
}
