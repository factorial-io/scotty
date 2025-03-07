use std::path::Path;

use tracing::debug;

use crate::{api::ws::broadcast_message, docker::docker_compose::run_task};

use super::context::Context;

pub async fn run_task_and_wait(
    context: &Context,
    docker_compose_path: &Path,
    command: &str,
    args: &[&str],
    env: &std::collections::HashMap<String, String>,
    msg: &str,
) -> anyhow::Result<()> {
    debug!("Running {} ", msg);

    let task_details = run_task(
        &context.app_state,
        docker_compose_path,
        command,
        args,
        env,
        context.task.clone(),
    )
    .await?;
    broadcast_message(
        &context.app_state,
        crate::api::message::WebSocketMessage::TaskInfoUpdated(task_details.clone()),
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

        broadcast_message(
            &context.app_state,
            crate::api::message::WebSocketMessage::TaskInfoUpdated(task.clone()),
        )
        .await;
    }

    let task = context
        .app_state
        .task_manager
        .get_task_details(&task_details.id)
        .await
        .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
    if let Some(last_exit_code) = task.last_exit_code {
        if last_exit_code != 0 {
            return Err(anyhow::anyhow!(
                "{} failed with exit code {}",
                msg,
                last_exit_code
            ));
        }
    }
    debug!("{} finished", msg);
    broadcast_message(
        &context.app_state,
        crate::api::message::WebSocketMessage::TaskInfoUpdated(task.clone()),
    )
    .await;

    Ok(())
}
