use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::app_state::SharedAppState;
use crate::tasks::timed_buffer::TimedBuffer;
use scotty_core::websocket::message::WebSocketMessage;
use scotty_types::{OutputLine, TaskOutputData};

use thiserror::Error;

/// Error types for task output streaming operations
#[derive(Error, Debug, Clone)]
#[allow(dead_code)]
pub enum TaskOutputStreamError {
    #[error("Task '{task_id}' not found")]
    TaskNotFound { task_id: Uuid },

    #[error("Failed to send command to stream: {reason}")]
    CommandSendFailed { reason: String },

    #[error("Task output access failed: {reason}")]
    OutputAccessFailed { reason: String },
}

/// Result type alias for task output streaming operations
pub type TaskOutputStreamResult<T> = Result<T, TaskOutputStreamError>;

/// Commands that can be sent to a task output stream
#[derive(Debug)]
#[allow(dead_code)]
pub enum TaskOutputStreamCommand {
    Stop,
}

/// Service for managing task output streams
#[derive(Debug, Clone)]
pub struct TaskOutputStreamingService {
    // No persistent state needed - each stream is independent
}

impl Default for TaskOutputStreamingService {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskOutputStreamingService {
    pub fn new() -> Self {
        Self {}
    }

    /// Start streaming task output to a client
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

        // Check if task exists
        let task_output = app_state
            .task_manager
            .get_task_output(&task_id)
            .await
            .ok_or(TaskOutputStreamError::TaskNotFound { task_id })?;

        // Get task state
        let task_details = app_state
            .task_manager
            .get_task_details(&task_id)
            .await
            .ok_or(TaskOutputStreamError::TaskNotFound { task_id })?;

        // Send stream started notification
        let _ = app_state
            .messenger
            .send_to_client(
                client_id,
                WebSocketMessage::TaskOutputStreamStarted {
                    task_id,
                    total_lines: task_output.total_lines_processed,
                },
            )
            .await;

        // Create channel for controlling the stream
        let (_tx, mut rx) = mpsc::channel::<TaskOutputStreamCommand>(1);

        // Clone what we need for the async task
        let app_state = app_state.clone();
        let output_collection_active = task_details.output_collection_active;

        // Start the streaming task
        crate::metrics::spawn_instrumented(async move {
            crate::metrics::tasks::record_stream_started();
            info!(
                "Task output streaming task started for task {} (output_collection_active: {})",
                task_id, output_collection_active
            );

            let mut last_sent_sequence = 0u64;
            let mut buffer = TimedBuffer::new(10, 100); // 10 lines or 100ms

            // Send historical data if requested
            if from_beginning {
                if let Some(task_output) = app_state.task_manager.get_task_output(&task_id).await {
                    let historical_lines: Vec<OutputLine> = task_output.lines.into_iter().collect();

                    if !historical_lines.is_empty() {
                        info!(
                            "Sending {} historical lines for task {}",
                            historical_lines.len(),
                            task_id
                        );

                        // Send historical lines in batches
                        const BATCH_SIZE: usize = 1000;
                        let chunks = historical_lines.chunks(BATCH_SIZE);
                        let total_chunks = chunks.len();

                        for (i, chunk) in chunks.enumerate() {
                            let is_last_batch = i == total_chunks - 1;
                            let chunk_len = chunk.len() as u64;
                            let _ = app_state
                                .messenger
                                .send_to_client(
                                    client_id,
                                    WebSocketMessage::TaskOutputData(TaskOutputData {
                                        task_id,
                                        lines: chunk.to_vec(),
                                        is_historical: true,
                                        has_more: !is_last_batch,
                                    }),
                                )
                                .await;
                            crate::metrics::tasks::record_output_lines(chunk_len);
                        }

                        // Update last sent sequence to the last historical line
                        if let Some(last_line) = historical_lines.last() {
                            last_sent_sequence = last_line.sequence + 1;
                        }
                    }
                }
            }

            // If output collection is not active, we're done after sending historical data
            if !output_collection_active {
                info!(
                    "Output collection inactive for task {}, ending stream after historical data",
                    task_id
                );

                let _ = app_state
                    .messenger
                    .send_to_client(
                        client_id,
                        WebSocketMessage::TaskOutputStreamEnded {
                            task_id,
                            reason: "Output collection completed".to_string(),
                        },
                    )
                    .await;
                crate::metrics::tasks::record_stream_ended();
                return;
            }

            // Set up timing for live streaming
            let flush_interval = tokio::time::Duration::from_millis(100);
            let mut flush_timer = tokio::time::interval(flush_interval);
            flush_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            let poll_interval = tokio::time::Duration::from_millis(100);
            let mut poll_timer = tokio::time::interval(poll_interval);
            poll_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            info!(
                "Starting live streaming for task {} from sequence {}",
                task_id, last_sent_sequence
            );

            // Live streaming loop
            loop {
                tokio::select! {
                    // Check for control commands
                    Some(cmd) = rx.recv() => {
                        match cmd {
                            TaskOutputStreamCommand::Stop => {
                                info!("Stopping task output stream {} by external request", task_id);
                                break;
                            }
                        }
                    }
                    // Check if we should flush the buffer based on time
                    _ = flush_timer.tick() => {
                        if buffer.has_data() && buffer.should_flush() {
                            let lines_to_send = buffer.flush();
                            let lines_count = lines_to_send.len();
                            if lines_count > 0 {
                                let _ = app_state.messenger.send_to_client(
                                    client_id,
                                    WebSocketMessage::TaskOutputData(TaskOutputData {
                                        task_id,
                                        lines: lines_to_send,
                                        is_historical: false,
                                        has_more: false,
                                    }),
                                ).await;
                                crate::metrics::tasks::record_output_lines(lines_count as u64);
                            }
                        }
                    }
                    // Poll for new task output
                    _ = poll_timer.tick() => {
                        // Check if output collection is still active
                        if let Some(task_details) = app_state.task_manager.get_task_details(&task_id).await {
                            if !task_details.output_collection_active {
                                info!("Output collection disabled for task {}, ending stream", task_id);
                                break;
                            }
                        } else {
                            warn!("Task {} no longer exists, ending stream", task_id);
                            break;
                        }

                        // Check for new output
                        if let Some(_task_output) = app_state.task_manager.get_task_output(&task_id).await {
                            let details_guard = {
                                // Get the processes lock and clone the details Arc
                                let processes = app_state.task_manager.processes.read().await;
                                processes.get(&task_id).map(|task_state| task_state.details.clone())
                            };

                            if let Some(details_arc) = details_guard {
                                let details = details_arc.read().await;
                                let output = &details.output;

                                // Find new lines since last_sent_sequence
                                let new_lines: Vec<OutputLine> = output
                                    .lines
                                    .iter()
                                    .filter(|line| line.sequence >= last_sent_sequence)
                                    .cloned()
                                    .collect();

                                if !new_lines.is_empty() {
                                    for line in &new_lines {
                                        buffer.push(line.clone());
                                    }

                                    // Update last sent sequence
                                    if let Some(last_line) = new_lines.last() {
                                        last_sent_sequence = last_line.sequence + 1;
                                    }

                                    // Send immediately if buffer should flush
                                    if buffer.should_flush() {
                                        let lines_to_send = buffer.flush();
                                        let lines_count = lines_to_send.len() as u64;
                                        let _ = app_state.messenger.send_to_client(
                                            client_id,
                                            WebSocketMessage::TaskOutputData(TaskOutputData {
                                                task_id,
                                                lines: lines_to_send,
                                                is_historical: false,
                                                has_more: false,
                                            }),
                                        ).await;
                                        crate::metrics::tasks::record_output_lines(lines_count);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Send any remaining buffered lines
            if buffer.has_data() {
                let lines_to_send = buffer.flush();
                let lines_count = lines_to_send.len() as u64;
                let _ = app_state
                    .messenger
                    .send_to_client(
                        client_id,
                        WebSocketMessage::TaskOutputData(TaskOutputData {
                            task_id,
                            lines: lines_to_send,
                            is_historical: false,
                            has_more: false,
                        }),
                    )
                    .await;
                crate::metrics::tasks::record_output_lines(lines_count);
            }

            // Send stream ended message
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

            crate::metrics::tasks::record_stream_ended();
            info!(
                "Task output stream for task {} ended and cleaned up",
                task_id
            );
        });

        Ok(())
    }
}
