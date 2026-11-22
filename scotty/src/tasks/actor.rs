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
use scotty_types::{StepState, TransferProgress};
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{mpsc, watch};
use tokio::time::{Duration, Instant};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::api::websocket::WebSocketMessenger;
use crate::metrics;
use crate::services::AuthorizationService;

/// Immutable view of a task, published after every applied event.
pub type Snapshot = Arc<TaskDetails>;

const MAILBOX_CAPACITY: usize = 1024;

/// Minimum time between two progress-only `TaskInfoUpdated` broadcasts.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

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
    /// The operation exceeded the maximum operation duration.
    TimedOut,
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
    /// A declared step changed its state. Applies to the first step called
    /// `name` that is still pending or running; ignored if there is none, so
    /// a step function can report steps an operation did not declare.
    Step { name: String, state: StepState },
    /// Progress within the running step: its own fraction (0 to 1), a status
    /// line and the image transfers.
    Progress {
        step_fraction: f32,
        status: String,
        transfers: Vec<TransferProgress>,
    },
    /// The operation ended. Applied once; later terminations are ignored.
    Terminate(Outcome),
}

/// Spawn the actor for `details`. Returns the owning handle and a snapshot
/// receiver for readers.
pub fn spawn(
    details: TaskDetails,
    messenger: WebSocketMessenger,
    auth: Arc<AuthorizationService>,
) -> (TaskHandle, watch::Receiver<Snapshot>) {
    let id = details.id;
    let (tx, mailbox) = mpsc::channel(MAILBOX_CAPACITY);
    let (snapshot, snapshot_rx) = watch::channel(Arc::new(details.clone()));
    let actor = TaskActor {
        details,
        snapshot,
        mailbox,
        messenger,
        auth,
        step_fraction: 0.0,
        last_broadcast: None,
        pending_broadcast: false,
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
    auth: Arc<AuthorizationService>,
    /// Progress within the running step.
    step_fraction: f32,
    last_broadcast: Option<Instant>,
    /// A throttled change is waiting for its trailing broadcast.
    pending_broadcast: bool,
}

/// What an applied event asks of `publish`.
#[derive(Debug, PartialEq)]
enum Push {
    /// Update the snapshot only (output lines are streamed separately).
    Snapshot,
    /// Update the snapshot and broadcast now.
    Now,
    /// Broadcast at most every `PROGRESS_INTERVAL`, leading and trailing edge.
    Throttled,
}

impl TaskActor {
    async fn run(mut self) {
        loop {
            let deadline = self
                .last_broadcast
                .filter(|_| self.pending_broadcast)
                .map(|t| t + PROGRESS_INTERVAL);
            let event = match deadline {
                Some(deadline) => tokio::select! {
                    event = self.mailbox.recv() => event,
                    _ = tokio::time::sleep_until(deadline) => {
                        self.publish(true).await;
                        continue;
                    }
                },
                None => self.mailbox.recv().await,
            };
            let Some(event) = event else { break };
            match self.apply(event) {
                Push::Snapshot => self.publish(false).await,
                Push::Now => self.publish(true).await,
                Push::Throttled => {
                    if self
                        .last_broadcast
                        .is_none_or(|t| t.elapsed() >= PROGRESS_INTERVAL)
                    {
                        self.publish(true).await;
                    } else {
                        // Readers get it with the trailing broadcast; saves a
                        // snapshot clone per progress event.
                        self.pending_broadcast = true;
                    }
                }
            }
        }
        if self.pending_broadcast {
            self.publish(true).await;
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

    /// Fold one event into the details.
    fn apply(&mut self, event: TaskEvent) -> Push {
        match event {
            TaskEvent::StepStarted { command } => {
                self.details.command = command;
                Push::Now
            }
            TaskEvent::Output(lines) => {
                for (stream, line) in lines {
                    self.details.output.add_line(stream, line);
                }
                Push::Snapshot
            }
            TaskEvent::SubprocessExited { exit_code } => {
                self.details.last_exit_code = exit_code;
                Push::Now
            }
            TaskEvent::Step { name, state } => self.set_step(&name, state),
            TaskEvent::Progress {
                step_fraction,
                status,
                transfers,
            } => {
                if self.details.state != State::Running {
                    return Push::Snapshot;
                }
                self.step_fraction = step_fraction.clamp(0.0, 1.0);
                self.details.progress.status = status;
                self.details.progress.transfers = transfers;
                self.update_fraction();
                Push::Throttled
            }
            TaskEvent::Terminate(outcome) => {
                if self.terminate(outcome) {
                    Push::Now
                } else {
                    Push::Snapshot
                }
            }
        }
    }

    fn set_step(&mut self, name: &str, state: StepState) -> Push {
        if self.details.state != State::Running {
            return Push::Snapshot;
        }
        let progress = &mut self.details.progress;
        let Some(step) = progress
            .steps
            .iter_mut()
            .find(|s| s.name == name && matches!(s.state, StepState::Pending | StepState::Running))
        else {
            return Push::Snapshot;
        };
        step.state = state;
        self.step_fraction = 0.0;
        progress.transfers.clear();
        if state == StepState::Running {
            progress.status = name.to_string();
        }
        self.update_fraction();
        Push::Now
    }

    /// Weighted share of the steps done (or skipped), plus the running step's
    /// own fraction. Never decreases while the task is running.
    fn update_fraction(&mut self) {
        let steps = &self.details.progress.steps;
        let total: u32 = steps.iter().map(|s| s.weight).sum();
        if total == 0 {
            return;
        }
        let done: f32 = steps
            .iter()
            .map(|s| match s.state {
                StepState::Done | StepState::Skipped => s.weight as f32,
                StepState::Running => s.weight as f32 * self.step_fraction,
                StepState::Pending | StepState::Failed => 0.0,
            })
            .sum();
        let fraction = (done / total as f32).min(1.0);
        let progress = &mut self.details.progress;
        progress.fraction = progress.fraction.max(fraction);
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
        self.details.output.add_line(stream, status.clone());
        let failed = state == State::Failed;
        let progress = &mut self.details.progress;
        progress.status = status;
        progress.transfers.clear();
        if failed {
            // A step cut off by an abort or a time limit failed with the task.
            for step in &mut progress.steps {
                if step.state == StepState::Running {
                    step.state = StepState::Failed;
                }
            }
        } else {
            progress.fraction = 1.0;
        }
        self.details.state = state;
        self.details.output_collection_active = false;
        let finish_time = jiff::Timestamp::now();
        self.details.finish_time = Some(finish_time);

        let duration_secs = finish_time
            .duration_since(self.details.start_time)
            .as_millis() as f64
            / 1000.0;
        metrics::metrics().record_task_finished(duration_secs, failed);
        true
    }

    async fn publish(&mut self, broadcast: bool) {
        let snapshot = Arc::new(self.details.clone());
        self.snapshot.send_replace(snapshot.clone());
        if broadcast {
            self.last_broadcast = Some(Instant::now());
            self.pending_broadcast = false;
            // Only clients allowed to see this task get the update.
            let message = WebSocketMessage::TaskInfoUpdated(Box::new((*snapshot).clone()));
            for (client_id, user) in self.messenger.authenticated_clients().await {
                let user_id = AuthorizationService::get_user_id_for_authorization(&user);
                if self.auth.can_view_task(&user_id, &snapshot).await {
                    let _ = self
                        .messenger
                        .send_to_client(client_id, message.clone())
                        .await;
                }
            }
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

    pub async fn step(&self, name: impl Into<String>, state: StepState) {
        self.send(TaskEvent::Step {
            name: name.into(),
            state,
        })
        .await;
    }

    pub async fn progress(
        &self,
        step_fraction: f32,
        status: impl Into<String>,
        transfers: Vec<TransferProgress>,
    ) {
        self.send(TaskEvent::Progress {
            step_fraction,
            status: status.into(),
            transfers,
        })
        .await;
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
impl TaskWriter {
    /// A writer whose events go to the returned receiver instead of an actor.
    pub(crate) fn detached() -> (Self, mpsc::Receiver<TaskEvent>) {
        let (tx, rx) = mpsc::channel(MAILBOX_CAPACITY);
        (
            Self {
                id: Uuid::new_v4(),
                tx,
            },
            rx,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::basic_auth::CurrentUser;
    use crate::api::test_utils::create_test_websocket_messenger;
    use crate::app_state::WebSocketClient;
    use axum::extract::ws::Message;
    use std::time::Duration;
    use tokio::sync::broadcast;

    async fn test_auth() -> Arc<AuthorizationService> {
        Arc::new(AuthorizationService::create_fallback_service(None).await)
    }

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
        let (handle, mut rx) = spawn(
            TaskDetails::default(),
            create_test_websocket_messenger(),
            test_auth().await,
        );
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
        let (handle, mut rx) = spawn(
            TaskDetails::default(),
            create_test_websocket_messenger(),
            test_auth().await,
        );
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
        let (handle, mut rx) = spawn(
            TaskDetails::default(),
            create_test_websocket_messenger(),
            test_auth().await,
        );
        drop(handle);
        let task = settled(&mut rx, |t| t.state != State::Running).await;
        assert_eq!(task.state, State::Failed);
        assert!(statuses(&task)[0].contains("aborted unexpectedly"));
    }

    #[tokio::test]
    async fn output_order_is_preserved_and_subprocess_exit_keeps_running() {
        let (handle, mut rx) = spawn(
            TaskDetails::default(),
            create_test_websocket_messenger(),
            test_auth().await,
        );
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

    /// Connect a client, authenticated as `user` if given.
    async fn connect(
        messenger: &WebSocketMessenger,
        user: Option<&str>,
    ) -> broadcast::Receiver<Message> {
        let (tx, rx) = broadcast::channel(100);
        let id = Uuid::new_v4();
        messenger.add_client(id, WebSocketClient::new(tx)).await;
        if let Some(user) = user {
            let user = CurrentUser {
                email: user.to_string(),
                name: user.to_string(),
                picture: None,
                access_token: None,
            };
            messenger.authenticate_client(id, user).await.unwrap();
        }
        rx
    }

    fn task_updates(rx: &mut broadcast::Receiver<Message>) -> usize {
        std::iter::from_fn(|| rx.try_recv().ok())
            .filter(|m| matches!(m, Message::Text(t) if t.contains("TaskInfoUpdated")))
            .count()
    }

    /// Authorization from the test fixture, with `app-a` in scope `client-a`.
    async fn fixture_auth() -> (Arc<AuthorizationService>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::copy(
            std::path::Path::new("tests/fixtures/policy").join("policy.yaml"),
            dir.path().join("policy.yaml"),
        )
        .unwrap();
        let auth = Arc::new(
            AuthorizationService::new(dir.path().to_str().unwrap())
                .await
                .unwrap(),
        );
        auth.set_app_scopes("app-a", vec!["client-a".to_string()])
            .await
            .unwrap();
        (auth, dir)
    }

    /// Waits until `client` receives a `TaskInfoUpdated`.
    async fn expect_task_update(client: &mut broadcast::Receiver<Message>, who: &str) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while !matches!(client.recv().await.unwrap(), Message::Text(t) if t.contains("TaskInfoUpdated")) {}
        })
        .await
        .unwrap_or_else(|_| panic!("{who} got no task update"));
    }

    #[tokio::test]
    async fn task_updates_reach_only_clients_allowed_to_see_the_task() {
        let (auth, _dir) = fixture_auth().await;

        let messenger = create_test_websocket_messenger();
        let mut anonymous = connect(&messenger, None).await;
        let mut allowed = connect(&messenger, Some("identifier:client-a")).await;
        let mut other = connect(&messenger, Some("identifier:someone-else")).await;

        let details = TaskDetails {
            app_name: Some("app-a".to_string()),
            ..TaskDetails::default()
        };
        let (handle, mut rx) = spawn(details, messenger, auth);
        handle.terminate(Outcome::finished("done")).await;
        settled(&mut rx, |t| t.state != State::Running).await;

        // The snapshot is published before the WebSocket send, so wait for it.
        expect_task_update(&mut allowed, "allowed client").await;
        assert_eq!(task_updates(&mut anonymous), 0);
        assert_eq!(task_updates(&mut other), 0);
    }

    /// A task with steps `a` (weight 40) and `b` (weight 10), `a` running.
    fn planned() -> TaskDetails {
        let step = |name: &str, state, weight| scotty_types::StepProgress {
            name: name.to_string(),
            state,
            weight,
        };
        let mut details = TaskDetails {
            app_name: Some("app-a".to_string()),
            ..TaskDetails::default()
        };
        details.progress.steps = vec![
            step("a", StepState::Running, 40),
            step("b", StepState::Pending, 10),
        ];
        details
    }

    #[tokio::test]
    async fn fraction_never_decreases_and_is_one_when_finished() {
        let (handle, mut rx) = spawn(
            planned(),
            create_test_websocket_messenger(),
            test_auth().await,
        );
        let w = handle.writer();
        w.progress(0.5, "half", vec![]).await;
        settled(&mut rx, |t| t.progress.status == "half").await;
        // A pull that discovers more layers reports a lower step fraction.
        w.progress(0.2, "more layers", vec![]).await;
        let task = settled(&mut rx, |t| t.progress.status == "more layers").await;
        assert_eq!(task.progress.fraction, 0.4);

        w.step("a", StepState::Done).await;
        w.step("b", StepState::Running).await;
        let task = settled(&mut rx, |t| t.progress.status == "b").await;
        assert_eq!(task.progress.fraction, 0.8);
        assert_eq!(task.progress.steps[0].state, StepState::Done);
        assert_eq!(task.progress.steps[1].state, StepState::Running);

        handle.terminate(Outcome::finished("done")).await;
        let task = settled(&mut rx, |t| t.state != State::Running).await;
        assert_eq!(task.progress.fraction, 1.0);
        assert_eq!(task.progress.status, "done");
    }

    #[tokio::test]
    async fn failed_task_keeps_its_fraction() {
        let (handle, mut rx) = spawn(
            planned(),
            create_test_websocket_messenger(),
            test_auth().await,
        );
        handle.writer().progress(0.5, "half", vec![]).await;
        handle
            .terminate(Outcome::failed(FailureKind::TimedOut, "too slow"))
            .await;
        let task = settled(&mut rx, |t| t.state != State::Running).await;
        assert_eq!(task.state, State::Failed);
        assert_eq!(task.progress.fraction, 0.4);
        // The step that was cut off failed; the next one never ran.
        assert_eq!(task.progress.steps[0].state, StepState::Failed);
        assert_eq!(task.progress.steps[1].state, StepState::Pending);
    }

    /// 200 progress events within a second: at most four broadcasts in any
    /// second, plus one per step change and the termination.
    #[tokio::test]
    async fn progress_broadcasts_are_throttled() {
        let (auth, _dir) = fixture_auth().await;
        let messenger = create_test_websocket_messenger();
        let mut client = connect(&messenger, Some("identifier:client-a")).await;
        let received = tokio::spawn(async move {
            let mut received = Vec::new();
            while let Ok(Message::Text(text)) = client.recv().await {
                if text.contains("TaskInfoUpdated") {
                    received.push((Instant::now(), text.to_string()));
                    if text.contains("Finished") {
                        break;
                    }
                }
            }
            received
        });

        let (handle, _rx) = spawn(planned(), messenger, auth);
        let w = handle.writer();
        for i in 0..200 {
            w.progress(i as f32 / 200.0, format!("event {i}"), vec![])
                .await;
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        w.step("a", StepState::Done).await;
        w.step("b", StepState::Running).await;
        handle.terminate(Outcome::finished("done")).await;
        let received = tokio::time::timeout(Duration::from_secs(2), received)
            .await
            .unwrap()
            .unwrap();

        let progress: Vec<Instant> = received
            .iter()
            // Sent before the first step change.
            .filter(|(_, text)| text.contains(r#""name":"a","state":"running""#))
            .map(|(at, _)| *at)
            .collect();
        assert!(progress.len() >= 2, "leading and trailing edge");
        for window in progress.windows(5) {
            assert!(
                window[4] - window[0] >= Duration::from_millis(950),
                "more than four progress broadcasts within a second"
            );
        }
        // Each step change and the termination are pushed at once.
        let others = received.len() - progress.len();
        assert_eq!(others, 3, "{received:?}");
    }

    #[tokio::test]
    async fn action_updates_reach_only_requester_and_action_holders() {
        let (auth, _dir) = fixture_auth().await;

        let messenger = create_test_websocket_messenger();
        let mut requester = connect(&messenger, Some("identifier:requester")).await;
        let mut admin = connect(&messenger, Some("identifier:admin")).await;
        // `view` on app-a, but not `action_write`.
        let mut viewer = connect(&messenger, Some("identifier:client-a")).await;

        let details = TaskDetails {
            app_name: Some("app-a".to_string()),
            requested_by: Some("identifier:requester".to_string()),
            action_permission: Some("action_write".to_string()),
            ..TaskDetails::default()
        };
        let (handle, mut rx) = spawn(details, messenger, auth);
        handle.terminate(Outcome::finished("done")).await;
        settled(&mut rx, |t| t.state != State::Running).await;

        expect_task_update(&mut requester, "requester").await;
        expect_task_update(&mut admin, "admin").await;
        assert_eq!(task_updates(&mut viewer), 0);
    }
}
