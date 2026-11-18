//! Registry of task actors plus subprocess execution.
//!
//! The manager owns nothing but snapshot receivers: each task's state lives in
//! its actor (see [`crate::tasks::actor`]). Readers get immutable snapshots,
//! writers go through the `TaskWriter`/`TaskHandle` returned by `create_task`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use scotty_core::output::OutputStreamType;
use scotty_core::settings::scheduler_interval::SchedulerInterval;
use scotty_core::tasks::task_details::{State, TaskDetails};
use scotty_core::utils::secret::MaskedSecret;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{watch, RwLock};
use tracing::{debug, info, instrument};
use uuid::Uuid;

use crate::api::websocket::WebSocketMessenger;
use crate::metrics;
use crate::services::AuthorizationService;
use crate::tasks::actor::{self, Snapshot, TaskHandle, TaskWriter};
use crate::tasks::compose_progress::PullProgress;
use crate::tasks::timed_buffer::TimedBuffer;

/// How a subprocess's output is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Every line is task output.
    Plain,
    /// `docker compose --progress json pull`: events become the task's
    /// transfer progress plus readable lines; other lines are output.
    ComposeProgress,
    /// `docker compose --progress json up` for an app with `services`
    /// services: like `ComposeProgress`, plus started containers.
    ComposeUp { services: usize },
}

#[derive(Clone, Debug)]
pub struct TaskManager {
    tasks: Arc<RwLock<HashMap<Uuid, watch::Receiver<Snapshot>>>>,
    messenger: WebSocketMessenger,
    auth: Arc<AuthorizationService>,
}

impl TaskManager {
    pub fn new(messenger: WebSocketMessenger, auth: Arc<AuthorizationService>) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            messenger,
            auth,
        }
    }

    /// Spawn an actor for `details` and register it. The returned handle owns
    /// the task: dropping it without `terminate` fails the task.
    pub async fn create_task(
        &self,
        details: TaskDetails,
    ) -> (TaskHandle, watch::Receiver<Snapshot>) {
        let id = details.id;
        let (handle, rx) = actor::spawn(details, self.messenger.clone(), self.auth.clone());
        let mut tasks = self.tasks.write().await;
        tasks.insert(id, rx.clone());
        metrics::metrics().record_task_added(tasks.len());
        (handle, rx)
    }

    /// Subscribe to a task's snapshots.
    pub async fn subscribe(&self, id: &Uuid) -> Option<watch::Receiver<Snapshot>> {
        self.tasks.read().await.get(id).cloned()
    }

    pub async fn get_task_list(&self) -> Vec<TaskDetails> {
        self.tasks
            .read()
            .await
            .values()
            .map(|rx| (**rx.borrow()).clone())
            .collect()
    }

    pub async fn get_task_details(&self, id: &Uuid) -> Option<TaskDetails> {
        self.tasks
            .read()
            .await
            .get(id)
            .map(|rx| (**rx.borrow()).clone())
    }

    /// Run `cmd` as a step of the task behind `writer`. Output lines and the
    /// exit code are reported to the task; the task state is never changed
    /// here (that is the owner's job). Resolves to the exit code.
    ///
    /// `stdin`, if given, is written to the process's standard input followed
    /// by a newline, then stdin is closed. Without it, stdin is empty.
    #[allow(clippy::too_many_arguments)]
    pub fn start_process(
        &self,
        cwd: &Path,
        cmd: &str,
        args: &[&str],
        env: &HashMap<String, String>,
        stdin: Option<&MaskedSecret>,
        writer: TaskWriter,
        mode: OutputMode,
    ) -> tokio::task::JoinHandle<anyhow::Result<i32>> {
        let cwd = cwd.to_path_buf();
        let cmd = cmd.to_string();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let env = env.clone();
        let stdin = stdin.cloned();

        tokio::spawn(async move {
            info!(
                task_id = %writer.id(),
                "Starting process in {}: {} {}",
                cwd.display(),
                cmd,
                args.join(" ")
            );
            writer
                .step_started(format!("{} {}", cmd, args.join(" ")))
                .await;

            let result = run_process(&cwd, &cmd, &args, &env, stdin.as_ref(), &writer, mode).await;
            match &result {
                Ok(code) => writer.subprocess_exited(Some(*code)).await,
                Err(e) => {
                    writer.status_error(format!("Process failed: {e:#}")).await;
                    writer.subprocess_exited(None).await;
                }
            }
            result
        })
    }

    /// Drop terminal tasks whose finish time is older than the TTL.
    #[instrument(skip(self))]
    pub async fn run_cleanup_task(&self, interval: SchedulerInterval) {
        let ttl: chrono::Duration = interval.into();
        let now = chrono::Utc::now();
        let mut tasks = self.tasks.write().await;
        let before = tasks.len();
        tasks.retain(|_, rx| {
            let task = rx.borrow();
            task.state == State::Running
                || task
                    .finish_time
                    .is_none_or(|t| now.signed_duration_since(t) <= ttl)
        });
        if tasks.len() != before {
            debug!("Cleaned up {} finished tasks", before - tasks.len());
            metrics::metrics().record_task_cleanup(tasks.len());
        }
    }
}

async fn run_process(
    cwd: &Path,
    cmd: &str,
    args: &[String],
    env: &HashMap<String, String>,
    stdin: Option<&MaskedSecret>,
    writer: &TaskWriter,
    mode: OutputMode,
) -> anyhow::Result<i32> {
    let mut child = Command::new(cmd)
        .args(args)
        .envs(env)
        .current_dir(cwd)
        // An operation cancelled at its time limit drops this future; the
        // process must not keep running without the app's lock.
        .kill_on_drop(true)
        .stdin(if stdin.is_some() {
            std::process::Stdio::piped()
        } else {
            std::process::Stdio::null()
        })
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to start `{cmd}`: {e}"))?;

    if let Some(secret) = stdin {
        let mut pipe = child.stdin.take().expect("stdin is piped");
        let secret = secret.clone();
        let cmd = cmd.to_string();
        // Written alongside the output readers, so a process blocked on full
        // stdout cannot deadlock against this write.
        tokio::spawn(async move {
            // A process that exits without reading stdin breaks the pipe; its
            // exit code still decides the result, the warning names the cause.
            let written = async {
                pipe.write_all(secret.expose_secret().as_bytes()).await?;
                // Line-oriented readers such as `docker login --password-stdin`
                // and `read` expect a terminated line.
                pipe.write_all(b"\n").await
            }
            .await;
            if let Err(e) = written {
                tracing::warn!("writing stdin of `{cmd}` failed: {e}");
            }
            // Dropping the pipe closes stdin, so the process sees EOF.
        });
    }

    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    // Compose may print its events on either stream; both feed one pull.
    let pull = match mode {
        OutputMode::Plain => None,
        OutputMode::ComposeProgress => Some(Arc::default()),
        OutputMode::ComposeUp { services } => {
            Some(Arc::new(Mutex::new(PullProgress::up(services))))
        }
    };
    let out_pump = tokio::spawn(pump_output(
        writer.clone(),
        stdout,
        OutputStreamType::Stdout,
        pull.clone(),
    ));
    let err_pump = tokio::spawn(pump_output(
        writer.clone(),
        stderr,
        OutputStreamType::Stderr,
        pull,
    ));

    let status = child.wait().await?;
    // Both pipes close on exit; join so every line is in the actor before
    // the exit is reported.
    let _ = tokio::join!(out_pump, err_pump);

    status
        .code()
        .ok_or_else(|| anyhow::anyhow!("`{cmd}` was terminated by a signal"))
}

async fn pump_output(
    writer: TaskWriter,
    stream: impl AsyncRead + Unpin,
    stream_type: OutputStreamType,
    pull: Option<Arc<Mutex<PullProgress>>>,
) {
    const BATCH_SIZE: usize = 20;
    const FLUSH_INTERVAL_MS: u64 = 100;

    let mut reader = BufReader::new(stream).lines();
    let mut buffer = TimedBuffer::new(BATCH_SIZE, FLUSH_INTERVAL_MS);
    // Compose repeats unchanged progress many times; only changes go to the
    // task's bounded mailbox. Every change is sent, so the final state is too.
    let mut last_progress = None;

    loop {
        match reader.next_line().await {
            Ok(Some(line)) => {
                if let Some(pull) = &pull {
                    let update = {
                        let mut pull = pull.lock().unwrap_or_else(|e| e.into_inner());
                        pull.apply(&line).map(|readable| {
                            (readable, (pull.fraction(), pull.status(), pull.transfers()))
                        })
                    };
                    if let Some((readable, progress)) = update {
                        if last_progress.as_ref() != Some(&progress) {
                            let (fraction, status, transfers) = progress.clone();
                            writer.progress(fraction, status, transfers).await;
                            last_progress = Some(progress);
                        }
                        if let Some(readable) = readable {
                            buffer.push((OutputStreamType::Stdout, readable));
                        }
                        if buffer.should_flush() {
                            writer.output_lines(buffer.flush()).await;
                        }
                        continue;
                    }
                }
                buffer.push((stream_type, line));
                if buffer.should_flush() {
                    writer.output_lines(buffer.flush()).await;
                }
            }
            Ok(None) => break,
            Err(e) => {
                writer.output_lines(buffer.flush()).await;
                writer
                    .output(
                        OutputStreamType::Stderr,
                        format!("Error reading output: {}", e),
                    )
                    .await;
                return;
            }
        }
    }
    writer.output_lines(buffer.flush()).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::test_utils::create_test_websocket_messenger;
    use crate::tasks::actor::Outcome;
    use std::time::Duration;

    async fn test_auth() -> Arc<AuthorizationService> {
        Arc::new(AuthorizationService::create_fallback_service(None).await)
    }

    /// Repeated identical compose progress reaches the task once.
    #[tokio::test]
    async fn unchanged_compose_progress_is_sent_once() {
        let line = r#"{"id":"Image alpine:3.17","status":"Working","text":"Pulling"}"#;
        let input = format!("{line}\n").repeat(50);
        let (writer, mut events) = TaskWriter::detached();
        pump_output(
            writer,
            input.as_bytes(),
            OutputStreamType::Stdout,
            Some(Arc::default()),
        )
        .await;
        let mut progress = 0;
        while let Ok(event) = events.try_recv() {
            progress += matches!(event, actor::TaskEvent::Progress { .. }) as usize;
        }
        assert_eq!(progress, 1);
    }

    async fn wait_for(
        rx: &mut watch::Receiver<Snapshot>,
        pred: impl Fn(&TaskDetails) -> bool,
    ) -> Snapshot {
        tokio::time::timeout(Duration::from_secs(5), async {
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

    #[tokio::test]
    async fn subprocess_exit_does_not_finish_task() {
        let manager = TaskManager::new(create_test_websocket_messenger(), test_auth().await);
        let (handle, mut rx) = manager.create_task(TaskDetails::default()).await;

        let code = manager
            .start_process(
                Path::new("."),
                "sh",
                &["-c", "echo one; echo two >&2; exit 3"],
                &HashMap::new(),
                None,
                handle.writer().clone(),
                OutputMode::Plain,
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(code, 3);

        let task = wait_for(&mut rx, |t| t.last_exit_code == Some(3)).await;
        assert_eq!(task.state, State::Running);
        let contents: Vec<_> = task
            .output
            .lines
            .iter()
            .filter(|l| {
                matches!(
                    l.stream,
                    OutputStreamType::Stdout | OutputStreamType::Stderr
                )
            })
            .map(|l| l.content.as_str())
            .collect();
        assert!(
            contents.contains(&"one") && contents.contains(&"two"),
            "{contents:?}"
        );

        handle.terminate(Outcome::finished("done")).await;
        let task = wait_for(&mut rx, |t| t.state != State::Running).await;
        assert_eq!(task.state, State::Finished);
        assert_eq!(
            manager.get_task_details(&task.id).await.unwrap().state,
            State::Finished
        );
    }

    /// A secret given as stdin reaches the process but not the task's command;
    /// without one, stdin is empty, so a reader ends instead of hanging.
    #[tokio::test]
    async fn stdin_secret_reaches_process_only() {
        let manager = TaskManager::new(create_test_websocket_messenger(), test_auth().await);
        let secret = MaskedSecret::new("s3cr3t-value".into());
        let script = r#"read p; [ "$p" = "$EXPECTED" ] && echo match"#;
        let env = HashMap::from([("EXPECTED".to_string(), "s3cr3t-value".to_string())]);

        for (stdin, expected) in [(Some(&secret), 0), (None, 1)] {
            let (handle, mut rx) = manager.create_task(TaskDetails::default()).await;
            let code = manager
                .start_process(
                    Path::new("."),
                    "sh",
                    &["-c", script],
                    &env,
                    stdin,
                    handle.writer().clone(),
                    OutputMode::Plain,
                )
                .await
                .unwrap()
                .unwrap();
            assert_eq!(code, expected);

            let task = wait_for(&mut rx, |t| t.last_exit_code == Some(expected)).await;
            assert!(!task.command.contains("s3cr3t-value"), "{}", task.command);
            assert!(task
                .output
                .lines
                .iter()
                .all(|l| !l.content.contains("s3cr3t-value")));
            handle.terminate(Outcome::finished("done")).await;
        }
    }

    /// A process that exits without reading its stdin still reports its own
    /// exit code; the broken pipe does not turn into a failure.
    #[tokio::test]
    async fn unread_stdin_keeps_exit_code() {
        let manager = TaskManager::new(create_test_websocket_messenger(), test_auth().await);
        let secret = MaskedSecret::new("x".repeat(256 * 1024));
        let (handle, _rx) = manager.create_task(TaskDetails::default()).await;
        let code = manager
            .start_process(
                Path::new("."),
                "sh",
                &["-c", "exit 4"],
                &HashMap::new(),
                Some(&secret),
                handle.writer().clone(),
                OutputMode::Plain,
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(code, 4);
        handle.terminate(Outcome::finished("done")).await;
    }

    /// A pull's events become transfer progress and readable lines; other
    /// lines stay output.
    #[tokio::test]
    async fn compose_progress_output_becomes_transfers() {
        let manager = TaskManager::new(create_test_websocket_messenger(), test_auth().await);
        let (handle, mut rx) = manager.create_task(TaskDetails::default()).await;
        let code = manager
            .start_process(
                Path::new("."),
                "sh",
                &[
                    "-c",
                    "cat tests/fixtures/compose_pull_progress.jsonl >&2; echo not an event",
                ],
                &HashMap::new(),
                None,
                handle.writer().clone(),
                OutputMode::ComposeProgress,
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(code, 0);
        handle.terminate(Outcome::finished("done")).await;
        let task = wait_for(&mut rx, |t| t.state != State::Running).await;
        let output: Vec<_> = task
            .output
            .lines
            .iter()
            .map(|l| l.content.as_str())
            .collect();
        assert!(
            output.contains(&"Pulled image redis:7.0.15-alpine"),
            "{output:?}"
        );
        assert!(output.contains(&"not an event"));
        assert!(output.iter().all(|l| !l.starts_with('{')), "{output:?}");
    }

    #[tokio::test]
    async fn missing_binary_is_reported_not_panicked() {
        let manager = TaskManager::new(create_test_websocket_messenger(), test_auth().await);
        let (handle, mut rx) = manager.create_task(TaskDetails::default()).await;
        let result = manager
            .start_process(
                Path::new("."),
                "/definitely/not/here",
                &[],
                &HashMap::new(),
                None,
                handle.writer().clone(),
                OutputMode::Plain,
            )
            .await
            .unwrap();
        assert!(result.is_err());
        let task = wait_for(&mut rx, |t| {
            t.output
                .lines
                .iter()
                .any(|l| l.content.contains("failed to start"))
        })
        .await;
        assert_eq!(task.state, State::Running);
    }

    #[tokio::test]
    async fn cleanup_removes_only_expired_terminal_tasks() {
        let manager = TaskManager::new(create_test_websocket_messenger(), test_auth().await);
        let (running, _) = manager.create_task(TaskDetails::default()).await;
        let (done, mut rx) = manager.create_task(TaskDetails::default()).await;
        done.terminate(Outcome::finished("done")).await;
        wait_for(&mut rx, |t| t.state != State::Running).await;

        manager.run_cleanup_task(SchedulerInterval::Hours(1)).await;
        assert_eq!(manager.get_task_list().await.len(), 2);

        manager
            .run_cleanup_task(SchedulerInterval::Seconds(0))
            .await;
        let left = manager.get_task_list().await;
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].id, running.id());
    }
}
