//! Streams a task's output lines to one WebSocket client by following the
//! task actor's snapshot channel. The stream ends once the task is terminal
//! and every line has been sent, so the final status line is never lost.

use tracing::info;
use uuid::Uuid;

use crate::app_state::SharedAppState;
use scotty_core::tasks::task_details::State;
use scotty_core::websocket::message::WebSocketMessage;
use scotty_types::{OutputLine, TaskOutputData};

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum TaskOutputStreamError {
    #[error("Task '{task_id}' not found")]
    TaskNotFound { task_id: Uuid },
}

pub type TaskOutputStreamResult<T> = Result<T, TaskOutputStreamError>;

#[derive(Debug, Clone, Default)]
pub struct TaskOutputStreamingService;

impl TaskOutputStreamingService {
    pub fn new() -> Self {
        Self
    }

    pub async fn start_task_output_stream(
        &self,
        app_state: &SharedAppState,
        task_id: Uuid,
        client_id: Uuid,
        from_beginning: bool,
    ) -> TaskOutputStreamResult<()> {
        info!(
            "Starting task output stream for task {} to client {}, from_beginning: {}",
            task_id, client_id, from_beginning
        );

        let mut rx = app_state
            .task_manager
            .subscribe(&task_id)
            .await
            .ok_or(TaskOutputStreamError::TaskNotFound { task_id })?;
        let mut current = rx.borrow_and_update().clone();

        let _ = app_state
            .messenger
            .send_to_client(
                client_id,
                WebSocketMessage::TaskOutputStreamStarted {
                    task_id,
                    total_lines: current.output.total_lines_processed,
                },
            )
            .await;

        let app_state = app_state.clone();
        crate::metrics::spawn_instrumented(async move {
            crate::metrics::metrics().record_task_output_stream_started();

            let send = |lines: Vec<OutputLine>, is_historical: bool, has_more: bool| {
                let app_state = app_state.clone();
                async move {
                    let count = lines.len();
                    let _ = app_state
                        .messenger
                        .send_to_client(
                            client_id,
                            WebSocketMessage::TaskOutputData(TaskOutputData {
                                task_id,
                                lines,
                                is_historical,
                                has_more,
                            }),
                        )
                        .await;
                    crate::metrics::metrics().record_task_output_lines(count);
                }
            };

            // Sequence of the next line to send. Lines are appended in
            // sequence order, so everything below it has been sent.
            let mut next_sequence = if from_beginning {
                const BATCH_SIZE: usize = 1000;
                let chunks: Vec<_> = current.output.lines.chunks(BATCH_SIZE).collect();
                let total = chunks.len();
                for (i, chunk) in chunks.into_iter().enumerate() {
                    send(chunk.to_vec(), true, i + 1 < total).await;
                }
                current.output.lines.last().map_or(0, |l| l.sequence + 1)
            } else {
                current.output.lines.last().map_or(0, |l| l.sequence + 1)
            };

            loop {
                let start = current
                    .output
                    .lines
                    .partition_point(|l| l.sequence < next_sequence);
                let new_lines = &current.output.lines[start..];
                if let Some(last) = new_lines.last() {
                    next_sequence = last.sequence + 1;
                    send(new_lines.to_vec(), false, false).await;
                }
                if current.state != State::Running {
                    break;
                }
                // Err means the actor is gone; whatever we have is final.
                if rx.changed().await.is_err() {
                    break;
                }
                current = rx.borrow_and_update().clone();
            }

            let _ = app_state
                .messenger
                .send_to_client(
                    client_id,
                    WebSocketMessage::TaskOutputStreamEnded {
                        task_id,
                        reason: "Stream completed".to_string(),
                    },
                )
                .await;
            crate::metrics::metrics().record_task_output_stream_ended();
            info!("Task output stream for task {} ended", task_id);
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::WebSocketClient;
    use crate::tasks::actor::Outcome;
    use scotty_core::output::OutputStreamType;
    use scotty_core::tasks::task_details::TaskDetails;
    use std::time::Duration;

    /// Lines written after the stream starts, including the final status line,
    /// all arrive before `TaskOutputStreamEnded`.
    #[tokio::test]
    async fn stream_delivers_every_line_before_ending() {
        let app_state = crate::api::test_utils::create_test_app_state_with_config(
            "tests/test_bearer_auth",
            None,
        )
        .await;
        let client_id = Uuid::new_v4();
        let (tx, mut client_rx) = tokio::sync::broadcast::channel(64);
        app_state
            .messenger
            .add_client(client_id, WebSocketClient::new(tx))
            .await;

        let (handle, _) = app_state
            .task_manager
            .create_task(TaskDetails::default())
            .await;
        handle.writer().status("before").await;

        app_state
            .task_output_service
            .start_task_output_stream(&app_state, handle.id(), client_id, true)
            .await
            .unwrap();

        for i in 0..3 {
            handle
                .writer()
                .output(OutputStreamType::Stdout, format!("line {i}"))
                .await;
        }
        handle.terminate(Outcome::finished("done")).await;

        let mut contents = vec![];
        let mut ended = false;
        while !ended {
            let msg = tokio::time::timeout(Duration::from_secs(5), client_rx.recv())
                .await
                .expect("stream never ended")
                .unwrap();
            let text = match msg {
                axum::extract::ws::Message::Text(t) => t.to_string(),
                other => panic!("unexpected frame {other:?}"),
            };
            match serde_json::from_str::<WebSocketMessage>(&text).unwrap() {
                WebSocketMessage::TaskOutputData(data) => {
                    contents.extend(data.lines.into_iter().map(|l| l.content));
                }
                WebSocketMessage::TaskOutputStreamEnded { .. } => ended = true,
                _ => {}
            }
        }
        assert_eq!(contents, ["before", "line 0", "line 1", "line 2", "done"]);
    }
}
