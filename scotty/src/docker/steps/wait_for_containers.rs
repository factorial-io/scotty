use anyhow::Context as _;
use scotty_core::output::OutputStreamType;
use tracing::{debug, info, instrument, warn};

use crate::docker::helper::wait_for_containers_ready;

use super::context::Context;

/// Wait until none of the app's containers is still starting (Created or
/// Restarting). `timeout_seconds` defaults to 300.
#[instrument(skip_all, fields(app = %ctx.app_data.name))]
pub async fn wait_for_all_containers(
    ctx: &Context,
    timeout_seconds: Option<u64>,
) -> anyhow::Result<()> {
    let container_ids: Vec<String> = ctx
        .app_data
        .services
        .iter()
        .filter_map(|service| service.id.clone())
        .collect();

    if container_ids.is_empty() {
        warn!("No container IDs found for app {}", ctx.app_data.name);
        return Ok(());
    }

    let writer = ctx.task.writer();
    writer
        .output(
            OutputStreamType::Progress,
            format!("Waiting for {} containers to be ready", container_ids.len()),
        )
        .await;
    info!("Waiting for containers to be ready: {:?}", container_ids);

    let container_states =
        wait_for_containers_ready(&ctx.app_state, container_ids, timeout_seconds)
            .await
            .context("Failed to wait for containers to be ready")?;

    writer.status("All containers are ready").await;
    debug!("Container states: {:?}", container_states);
    Ok(())
}
