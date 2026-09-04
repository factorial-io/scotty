//! Registry of task actors plus subprocess execution.
//!
//! The manager owns nothing but snapshot receivers: each task's state lives in
//! its actor (see [`crate::tasks::actor`]). Readers get immutable snapshots,
//! writers go through the `TaskWriter`/`TaskHandle` returned by `create_task`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use scotty_core::output::OutputStreamType;
use scotty_core::settings::scheduler_interval::SchedulerInterval;
use scotty_core::tasks::task_details::{State, TaskDetails};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::{watch, RwLock};
use tracing::{debug, info, instrument};
use uuid::Uuid;

use crate::api::websocket::WebSocketMessenger;
use crate::metrics;
use crate::tasks::actor::{self, Snapshot, TaskHandle, TaskWriter};
use crate::tasks::timed_buffer::TimedBuffer;

#[derive(Clone, Debug)]
pub struct TaskManager {
    tasks: Arc<RwLock<HashMap<Uuid, watch::Receiver<Snapshot>>>>,
    messenger: WebSocketMessenger,
}

impl TaskManager {
    pub fn new(messenger: WebSocketMessenger) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            messenger,
        }
    }

    /// Spawn an actor for `details` and register it. The returned handle owns
    /// the task: dropping it without `terminate` fails the task.
    pub async fn create_task(
        &self,
        details: TaskDetails,
    ) -> (TaskHandle, watch::Receiver<Snapshot>) {
        let id = details.id;
        let (handle, rx) = actor::spawn(details, self.messenger.clone());
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
    pub fn start_process(
        &self,
        cwd: &Path,
        cmd: &str,
        args: &[&str],
        env: &HashMap<String, String>,
        writer: TaskWriter,
    ) -> tokio::task::JoinHandle<anyhow::Result<i32>> {
        let cwd = cwd.to_path_buf();
        let cmd = cmd.to_string();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let env = env.clone();

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

            let result = run_process(&cwd, &cmd, &args, &env, &writer).await;
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
    writer: &TaskWriter,
) -> anyhow::Result<i32> {
    let mut child = Command::new(cmd)
        .args(args)
        .envs(env)
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to start `{cmd}`: {e}"))?;

    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let out_pump = tokio::spawn(pump_output(
        writer.clone(),
        stdout,
        OutputStreamType::Stdout,
    ));
    let err_pump = tokio::spawn(pump_output(
        writer.clone(),
        stderr,
        OutputStreamType::Stderr,
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
) {
    const BATCH_SIZE: usize = 20;
    const FLUSH_INTERVAL_MS: u64 = 100;

    let mut reader = BufReader::new(stream).lines();
    let mut buffer = TimedBuffer::new(BATCH_SIZE, FLUSH_INTERVAL_MS);

    loop {
        match reader.next_line().await {
            Ok(Some(line)) => {
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
        let manager = TaskManager::new(create_test_websocket_messenger());
        let (handle, mut rx) = manager.create_task(TaskDetails::default()).await;

        let code = manager
            .start_process(
                Path::new("."),
                "sh",
                &["-c", "echo one; echo two >&2; exit 3"],
                &HashMap::new(),
                handle.writer().clone(),
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

    #[tokio::test]
    async fn missing_binary_is_reported_not_panicked() {
        let manager = TaskManager::new(create_test_websocket_messenger());
        let (handle, mut rx) = manager.create_task(TaskDetails::default()).await;
        let result = manager
            .start_process(
                Path::new("."),
                "/definitely/not/here",
                &[],
                &HashMap::new(),
                handle.writer().clone(),
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
        let manager = TaskManager::new(create_test_websocket_messenger());
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
