use std::path::Path;

use scotty_core::utils::secret::SecretHashMap;
use tracing::debug;

use crate::docker::docker_compose::run_task;

use super::context::Context;

pub async fn run_task_and_wait(
    context: &Context,
    docker_compose_path: &Path,
    command: &str,
    args: &[&str],
    env: &SecretHashMap,
    msg: &str,
) -> anyhow::Result<()> {
    debug!("Running {} ", msg);

    let task_id = context.task.read().await.id;

    let task_details = run_task(
        &context.app_state,
        docker_compose_path,
        command,
        args,
        env,
        context.task.clone(),
    )
    .await?;
    context
        .app_state
        .messenger
        .broadcast_to_all(
            scotty_core::websocket::message::WebSocketMessage::TaskInfoUpdated(
                task_details.clone(),
            ),
        )
        .await;

    let handle = context
        .app_state
        .task_manager
        .get_task_handle(&task_details.id)
        .await
        .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

    debug!("Waiting for {} to finish", msg);
    while !handle.read().await.is_finished() {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let task = context
            .app_state
            .task_manager
            .get_task_details(&task_details.id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        context
            .app_state
            .messenger
            .broadcast_to_all(
                scotty_core::websocket::message::WebSocketMessage::TaskInfoUpdated(task.clone()),
            )
            .await;
    }

    let task = context
        .app_state
        .task_manager
        .get_task_details(&task_details.id)
        .await
        .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

    // Add completion status based on exit code
    if let Some(last_exit_code) = task.last_exit_code {
        if last_exit_code != 0 {
            context
                .app_state
                .task_manager
                .add_task_status_error(
                    &task_id,
                    format!("Failed: {} (exit code {})", msg, last_exit_code),
                )
                .await;
            return Err(anyhow::anyhow!(
                "{} failed with exit code {}",
                msg,
                last_exit_code
            ));
        }
    }

    context
        .app_state
        .task_manager
        .add_task_status(&task_id, format!("Completed: {}", msg))
        .await;

    debug!("{} finished", msg);
    context
        .app_state
        .messenger
        .broadcast_to_all(
            scotty_core::websocket::message::WebSocketMessage::TaskInfoUpdated(task.clone()),
        )
        .await;

    Ok(())
}
