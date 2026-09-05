//! One actor per task owns its `TaskDetails`.
//!
//! Pattern: Alice Ryhl, "Actors with Tokio". An owned struct runs in its own
//! tokio task, receives `TaskEvent`s over a bounded mpsc mailbox, folds them
//! into the details and publishes an immutable snapshot on a `watch` channel.
//! Nobody else holds a reference to the details, so there is no lock, no
//! ordering hazard between the last output line and the terminal state, and
//! the terminal transition happens exactly once by construction.
//!
//! Producers come in two flavours:
//! - [`TaskWriter`] (Clone): output lines, status messages, subprocess exits.
//! - [`TaskHandle`] (not Clone): the only way to terminate the task. Dropping
//!   it without terminating fails the task, which is what turns a panic or a
//!   swallowed error into a visible `Failed` instead of a task stuck `Running`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use scotty_core::output::OutputStreamType;
use scotty_core::tasks::task_details::{State, TaskDetails};
use scotty_core::websocket::message::WebSocketMessage;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{mpsc, watch};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::api::websocket::WebSocketMessenger;
use crate::metrics;

/// Immutable view of a task, published after every applied event.
pub type Snapshot = Arc<TaskDetails>;

const MAILBOX_CAPACITY: usize = 1024;

/// Why a task failed. Kept structured internally so a future consumer can
/// branch on it; the wire only carries the rendered status line for now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureKind {
    /// A step returned an error.
    StepFailed,
    /// The post-operation app data refresh failed.
    RefreshFailed,
    /// The operation ended without terminating its task (panic, dropped handle).
    Aborted,
}

#[derive(Debug, Clone)]
pub enum Outcome {
    Finished { status: String },
    Failed { kind: FailureKind, status: String },
}

impl Outcome {
    pub fn finished(status: impl Into<String>) -> Self {
        Self::Finished {
            status: status.into(),
        }
    }

    pub fn failed(kind: FailureKind, status: impl Into<String>) -> Self {
        Self::Failed {
            kind,
            status: status.into(),
        }
    }

    fn aborted() -> Self {
        Self::failed(
            FailureKind::Aborted,
            "Operation aborted unexpectedly (internal error)",
        )
    }
}

#[derive(Debug)]
pub enum TaskEvent {
    /// A new step (typically a subprocess) started; shown as the task command.
    StepStarted { command: String },
    /// Output lines, in order.
    Output(Vec<(OutputStreamType, String)>),
    /// A subprocess of this task exited. Never changes the task state.
    SubprocessExited { exit_code: Option<i32> },
    /// The operation ended. Applied once; later terminations are ignored.
    Terminate(Outcome),
}

/// Spawn the actor for `details`. Returns the owning handle and a snapshot
/// receiver for readers.
pub fn spawn(
    details: TaskDetails,
    messenger: WebSocketMessenger,
) -> (TaskHandle, watch::Receiver<Snapshot>) {
    let id = details.id;
    let (tx, mailbox) = mpsc::channel(MAILBOX_CAPACITY);
    let (snapshot, snapshot_rx) = watch::channel(Arc::new(details.clone()));
    let actor = TaskActor {
        details,
        snapshot,
        mailbox,
        messenger,
    };
    tokio::spawn(actor.run());
    let handle = TaskHandle {
        writer: TaskWriter { id, tx },
        terminated: AtomicBool::new(false),
    };
    (handle, snapshot_rx)
}

struct TaskActor {
    details: TaskDetails,
    snapshot: watch::Sender<Snapshot>,
    mailbox: mpsc::Receiver<TaskEvent>,
    messenger: WebSocketMessenger,
}

impl TaskActor {
    async fn run(mut self) {
        while let Some(event) = self.mailbox.recv().await {
            let broadcast = self.apply(event);
            self.publish(broadcast).await;
        }
        // Every writer and the handle are gone. A task that is still Running
        // at this point can never be terminated by anyone else.
        if self.details.state == State::Running {
            warn!(
                task_id = %self.details.id,
                "All task producers dropped while Running; failing the task"
            );
            self.apply(TaskEvent::Terminate(Outcome::aborted()));
            self.publish(true).await;
        }
        debug!(task_id = %self.details.id, "Task actor stopped");
    }

    /// Fold one event into the details. Returns whether the change is worth a
    /// `TaskInfoUpdated` broadcast (output lines are streamed separately).
    fn apply(&mut self, event: TaskEvent) -> bool {
        match event {
            TaskEvent::StepStarted { command } => {
                self.details.command = command;
                true
            }
            TaskEvent::Output(lines) => {
                for (stream, line) in lines {
                    self.details.output.add_line(stream, line);
                }
                false
            }
            TaskEvent::SubprocessExited { exit_code } => {
                self.details.last_exit_code = exit_code;
                true
            }
            TaskEvent::Terminate(outcome) => self.terminate(outcome),
        }
    }

    fn terminate(&mut self, outcome: Outcome) -> bool {
        if self.details.state != State::Running {
            warn!(
                task_id = %self.details.id,
                state = ?self.details.state,
                ?outcome,
                "Ignoring termination of an already terminal task"
            );
            return false;
        }
        let (state, stream, status) = match outcome {
            Outcome::Finished { status } => (State::Finished, OutputStreamType::Status, status),
            Outcome::Failed { kind, status } => {
                debug!(task_id = %self.details.id, ?kind, "Task failed");
                (State::Failed, OutputStreamType::StatusError, status)
            }
        };
        // The status line lands in the same snapshot as the terminal state, so
        // no reader can observe one without the other.
        self.details.output.add_line(stream, status);
        let failed = state == State::Failed;
        self.details.state = state;
        self.details.output_collection_active = false;
        let finish_time = chrono::Utc::now();
        self.details.finish_time = Some(finish_time);

        let duration_secs = finish_time
            .signed_duration_since(self.details.start_time)
            .num_milliseconds() as f64
            / 1000.0;
        metrics::metrics().record_task_finished(duration_secs, failed);
        true
    }

    async fn publish(&self, broadcast: bool) {
        let snapshot = Arc::new(self.details.clone());
        self.snapshot.send_replace(snapshot.clone());
        if broadcast {
            self.messenger
                .broadcast_to_all(WebSocketMessage::TaskInfoUpdated((*snapshot).clone()))
                .await;
        }
    }
}

/// Cloneable producer of non-terminal events.
#[derive(Clone, Debug)]
pub struct TaskWriter {
    id: Uuid,
    tx: mpsc::Sender<TaskEvent>,
}

impl TaskWriter {
    pub fn id(&self) -> Uuid {
        self.id
    }

    async fn send(&self, event: TaskEvent) {
        if self.tx.send(event).await.is_err() {
            debug!(task_id = %self.id, "Task actor is gone; dropping event");
        }
    }

    pub async fn output(&self, stream: OutputStreamType, line: impl Into<String>) {
        self.send(TaskEvent::Output(vec![(stream, line.into())]))
            .await;
    }

    pub async fn output_lines(&self, lines: Vec<(OutputStreamType, String)>) {
        if !lines.is_empty() {
            self.send(TaskEvent::Output(lines)).await;
        }
    }

    pub async fn status(&self, message: impl Into<String>) {
        self.output(OutputStreamType::Status, message).await;
    }

    pub async fn status_error(&self, message: impl Into<String>) {
        self.output(OutputStreamType::StatusError, message).await;
    }

    pub async fn step_started(&self, command: impl Into<String>) {
        self.send(TaskEvent::StepStarted {
            command: command.into(),
        })
        .await;
    }

    pub async fn subprocess_exited(&self, exit_code: Option<i32>) {
        self.send(TaskEvent::SubprocessExited { exit_code }).await;
    }
}

/// Owner of a task. Not `Clone`: whoever holds it decides how the task ends.
#[derive(Debug)]
pub struct TaskHandle {
    writer: TaskWriter,
    terminated: AtomicBool,
}

impl TaskHandle {
    pub fn id(&self) -> Uuid {
        self.writer.id
    }

    pub fn writer(&self) -> &TaskWriter {
        &self.writer
    }

    /// Terminate the task. Returns `false` if it was already terminated
    /// through this handle; the actor ignores duplicates as well.
    pub async fn terminate(&self, outcome: Outcome) -> bool {
        if self.terminated.swap(true, Ordering::AcqRel) {
            warn!(task_id = %self.id(), ?outcome, "Task already terminated");
            return false;
        }
        self.writer.send(TaskEvent::Terminate(outcome)).await;
        true
    }
}

impl Drop for TaskHandle {
    fn drop(&mut self) {
        if self.terminated.swap(true, Ordering::AcqRel) {
            return;
        }
        // Reached on panic unwind or when an operation forgot to terminate.
        // Cannot await here, so try synchronously and fall back to a spawned
        // send if the mailbox happens to be full.
        let tx = self.writer.tx.clone();
        match tx.try_send(TaskEvent::Terminate(Outcome::aborted())) {
            Ok(()) | Err(TrySendError::Closed(_)) => {}
            Err(TrySendError::Full(event)) => {
                if let Ok(rt) = tokio::runtime::Handle::try_current() {
                    rt.spawn(async move {
                        let _ = tx.send(event).await;
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::test_utils::create_test_websocket_messenger;
    use std::time::Duration;

    async fn settled(
        rx: &mut watch::Receiver<Snapshot>,
        pred: impl Fn(&TaskDetails) -> bool,
    ) -> Snapshot {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if pred(&rx.borrow()) {
                    return rx.borrow().clone();
                }
                rx.changed().await.unwrap();
            }
        })
        .await
        .expect("snapshot never satisfied predicate")
    }

    fn statuses(task: &TaskDetails) -> Vec<&str> {
        task.output
            .lines
            .iter()
            .filter(|l| {
                matches!(
                    l.stream,
                    OutputStreamType::Status | OutputStreamType::StatusError
                )
            })
            .map(|l| l.content.as_str())
            .collect()
    }

    #[tokio::test]
    async fn terminal_state_and_status_line_arrive_together() {
        let (handle, mut rx) = spawn(TaskDetails::default(), create_test_websocket_messenger());
        handle.writer().status("step one").await;
        handle.terminate(Outcome::finished("done")).await;

        let task = settled(&mut rx, |t| t.state != State::Running).await;
        assert_eq!(task.state, State::Finished);
        assert!(task.finish_time.is_some());
        assert!(!task.output_collection_active);
        assert_eq!(statuses(&task), vec!["step one", "done"]);
    }

    #[tokio::test]
    async fn second_terminate_is_ignored() {
        let (handle, mut rx) = spawn(TaskDetails::default(), create_test_websocket_messenger());
        assert!(
            handle
                .terminate(Outcome::failed(FailureKind::StepFailed, "first"))
                .await
        );
        assert!(!handle.terminate(Outcome::finished("second")).await);
        let first = settled(&mut rx, |t| t.state != State::Running).await;

        // Push another event so a later snapshot exists, then compare.
        handle.writer().status("late line").await;
        let later = settled(&mut rx, |t| t.output.lines.len() > first.output.lines.len()).await;
        assert_eq!(later.state, State::Failed);
        assert_eq!(later.finish_time, first.finish_time);
        assert!(!statuses(&later).contains(&"second"));
    }

    #[tokio::test]
    async fn dropping_handle_without_terminate_fails_task() {
        let (handle, mut rx) = spawn(TaskDetails::default(), create_test_websocket_messenger());
        drop(handle);
        let task = settled(&mut rx, |t| t.state != State::Running).await;
        assert_eq!(task.state, State::Failed);
        assert!(statuses(&task)[0].contains("aborted unexpectedly"));
    }

    #[tokio::test]
    async fn output_order_is_preserved_and_subprocess_exit_keeps_running() {
        let (handle, mut rx) = spawn(TaskDetails::default(), create_test_websocket_messenger());
        let w = handle.writer().clone();
        for i in 0..50 {
            w.output(OutputStreamType::Stdout, format!("line {i}"))
                .await;
        }
        w.subprocess_exited(Some(0)).await;
        let task = settled(&mut rx, |t| t.last_exit_code == Some(0)).await;
        assert_eq!(task.state, State::Running);
        let contents: Vec<_> = task
            .output
            .lines
            .iter()
            .map(|l| l.content.as_str())
            .collect();
        assert_eq!(
            contents,
            (0..50).map(|i| format!("line {i}")).collect::<Vec<_>>()
        );
        handle.terminate(Outcome::finished("ok")).await;
    }
}
