#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::{collections::HashMap, sync::Arc};

use scotty_core::settings::{output::OutputSettings, scheduler_interval::SchedulerInterval};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};

use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument};

use uuid::Uuid;

use scotty_core::output::{OutputStreamType, TaskOutput};
use scotty_core::tasks::task_details::{State, TaskDetails, TaskState};

use crate::api::websocket::WebSocketMessenger;
use crate::metrics;
use crate::tasks::timed_buffer::TimedBuffer;

/// Helper function to add multiple lines to task output with a single write lock
async fn add_output_lines(
    details: &Arc<RwLock<TaskDetails>>,
    lines: Vec<(OutputStreamType, String)>,
    task_id: uuid::Uuid,
) {
    if lines.is_empty() {
        return;
    }

    let mut details = details.write().await;
    for (stream_type, line) in lines {
        details.output.add_line(stream_type, line);
    }
    debug!(
        "Added {} output lines to task {}",
        details.output.lines.len(),
        task_id
    );
}

/// Helper function to add a single line to task output
async fn add_output_line(
    details: &Arc<RwLock<TaskDetails>>,
    stream_type: OutputStreamType,
    line: String,
    task_id: uuid::Uuid,
) {
    add_output_lines(details, vec![(stream_type, line)], task_id).await;
}

/// Helper function to handle task completion consistently
async fn handle_task_completion(
    details: &Arc<RwLock<TaskDetails>>,
    _messenger: &WebSocketMessenger,
    _task_manager: &TaskManager,
    task_id: uuid::Uuid,
    exit_code: Result<i32, anyhow::Error>,
) {
    debug!("Handling task completion for task {}", task_id);
    let (state, status_code, _reason) = match exit_code {
        Ok(0) => (State::Finished, Some(0), "completed"),
        Ok(e) => (State::Failed, Some(e), "failed"),
        Err(_) => (State::Failed, None, "failed"),
    };

    TaskManager::set_task_finished(details, status_code, state.clone()).await;

    // With the new self-contained streaming approach, no cleanup is needed.
    // Streams detect task completion and terminate naturally.
    debug!("Task {} completed with status {:?}", task_id, state);
}

#[derive(Clone, Debug)]
pub struct TaskManager {
    pub processes: Arc<RwLock<HashMap<Uuid, TaskState>>>,
    messenger: WebSocketMessenger,
}

impl TaskManager {
    pub fn new(messenger: WebSocketMessenger) -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
            messenger,
        }
    }

    /// Helper: Get a task's details Arc with minimal lock time
    async fn get_details_arc(&self, uuid: &Uuid) -> Option<Arc<RwLock<TaskDetails>>> {
        let processes = self.processes.read().await;
        processes
            .get(uuid)
            .map(|task_state| task_state.details.clone())
    }

    /// Helper: Get all task details Arcs with minimal lock time
    async fn get_all_details_arcs(&self) -> Vec<Arc<RwLock<TaskDetails>>> {
        let processes = self.processes.read().await;
        processes
            .values()
            .map(|task_state| task_state.details.clone())
            .collect()
    }

    /// Helper: Modify a task's details with minimal lock time
    async fn modify_task_details<F>(&self, uuid: &Uuid, f: F) -> bool
    where
        F: FnOnce(&mut TaskDetails),
    {
        let Some(details_arc) = self.get_details_arc(uuid).await else {
            return false;
        };
        let mut details = details_arc.write().await;
        f(&mut details);
        true
    }

    pub async fn get_task_list(&self) -> Vec<TaskDetails> {
        let detail_arcs = self.get_all_details_arcs().await;

        let mut task_list = Vec::new();
        for details_arc in detail_arcs {
            let details = details_arc.read().await;
            task_list.push(details.clone());
        }
        task_list
    }

    pub async fn get_task_details(&self, uuid: &Uuid) -> Option<TaskDetails> {
        debug!("TaskManager: Getting task details for {}", uuid);

        let details_arc = self.get_details_arc(uuid).await?;
        let details = details_arc.read().await;
        Some(details.clone())
    }

    pub async fn get_task_output(&self, uuid: &Uuid) -> Option<TaskOutput> {
        debug!("TaskManager: Getting task output for {}", uuid);
        Some(self.get_task_details(uuid).await?.output)
    }

    /// Add a message to a task's output stream
    pub async fn add_task_message(
        &self,
        uuid: &Uuid,
        stream_type: OutputStreamType,
        message: String,
    ) -> bool {
        debug!("TaskManager: Adding message to task output for {}", uuid);
        self.modify_task_details(uuid, |details| {
            details.output.add_line(stream_type, message);
        })
        .await
    }

    /// Add an info message to a task's stdout
    pub async fn add_task_info(&self, uuid: &Uuid, message: String) -> bool {
        self.add_task_message(uuid, OutputStreamType::Stdout, message)
            .await
    }

    /// Add a status message to task output
    pub async fn add_task_status(&self, uuid: &Uuid, message: String) -> bool {
        self.add_task_message(uuid, OutputStreamType::Status, message)
            .await
    }

    /// Add an error status message to task output
    pub async fn add_task_status_error(&self, uuid: &Uuid, message: String) -> bool {
        self.add_task_message(uuid, OutputStreamType::StatusError, message)
            .await
    }

    /// Add a progress message to task output
    pub async fn add_task_progress(&self, uuid: &Uuid, message: String) -> bool {
        self.add_task_message(uuid, OutputStreamType::Progress, message)
            .await
    }

    /// Add a formatted progress message with step counter
    pub async fn add_task_step_progress(
        &self,
        uuid: &Uuid,
        current: u32,
        total: u32,
        message: String,
    ) -> bool {
        let progress_msg = format!("Step {}/{}: {}", current, total, message);
        self.add_task_progress(uuid, progress_msg).await
    }

    /// Add an informational message to task output
    pub async fn add_task_info_message(&self, uuid: &Uuid, message: String) -> bool {
        self.add_task_message(uuid, OutputStreamType::Info, message)
            .await
    }

    pub async fn get_task_handle(
        &self,
        uuid: &Uuid,
    ) -> Option<Arc<RwLock<tokio::task::JoinHandle<()>>>> {
        let processes = self.processes.read().await;
        let task_state = processes.get(uuid);
        task_state?;
        let task_state = task_state.unwrap();
        task_state.handle.clone()
    }

    /// Set task state to finished with completion details
    async fn set_task_finished(
        details: &Arc<RwLock<TaskDetails>>,
        exit_code: Option<i32>,
        state: State,
    ) {
        debug!(
            "TaskManager: Setting task finished for {}",
            details.read().await.id
        );

        let start_time = details.read().await.start_time;
        let mut details_guard = details.write().await;

        let finish_time = chrono::Utc::now();
        details_guard.last_exit_code = exit_code;
        details_guard.finish_time = Some(finish_time);
        details_guard.state = state.clone();

        // Record metrics
        let duration_secs = finish_time
            .signed_duration_since(start_time)
            .num_milliseconds() as f64
            / 1000.0;
        let failed = matches!(state, State::Failed);
        metrics::metrics().record_task_finished(duration_secs, failed);
    }

    pub async fn start_process(
        &self,
        cwd: &Path,
        cmd: &str,
        args: &[&str],
        env: &HashMap<String, String>,
        details: Arc<RwLock<TaskDetails>>,
    ) -> Uuid {
        self.start_process_with_settings(cwd, cmd, args, env, details, &OutputSettings::default())
            .await
    }

    pub async fn start_process_with_settings(
        &self,
        cwd: &Path,
        cmd: &str,
        args: &[&str],
        env: &HashMap<String, String>,
        details: Arc<RwLock<TaskDetails>>,
        _output_settings: &OutputSettings,
    ) -> Uuid {
        let cwd = cwd.to_path_buf();
        let cmd = cmd.to_string();
        let args = args.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        let env = env.clone();
        let id = details.read().await.id;

        {
            debug!("details write lock, setting state to running");
            let mut details = details.write().await;
            details.state = State::Running;
        }

        let handle = {
            let details = details.clone();
            let messenger = self.messenger.clone();
            let task_manager = self.clone(); // Clone the TaskManager for cleanup

            tokio::task::spawn(async move {
                info!(
                    "Starting process with uuid {} in folder {}: {:?} {}",
                    &id,
                    cwd.display(),
                    cmd,
                    args.join(" ")
                );
                let details = details.clone();
                let messenger_for_spawn = messenger.clone();
                let exit_result = spawn_process(
                    &cwd,
                    &cmd,
                    &args,
                    &env,
                    &details,
                    messenger_for_spawn.clone(),
                )
                .await;

                handle_task_completion(&details, &messenger, &task_manager, id, exit_result).await;
            })
        };
        {
            self.add_task(&id, details.clone(), Some(Arc::new(RwLock::new(handle))))
                .await;
        }

        id
    }

    pub async fn add_task(
        &self,
        id: &Uuid,
        details: Arc<RwLock<TaskDetails>>,
        handle: Option<Arc<RwLock<tokio::task::JoinHandle<()>>>>,
    ) {
        let mut processes = self.processes.write().await;
        let task_state = TaskState { details, handle };
        processes.insert(*id, task_state);

        metrics::metrics().record_task_added(processes.len());
    }

    pub async fn add_task_with_output(
        &self,
        id: &Uuid,
        details: Arc<RwLock<TaskDetails>>,
        handle: Option<Arc<RwLock<tokio::task::JoinHandle<()>>>>,
        _output: Arc<RwLock<TaskOutput>>,
    ) {
        // Output is now embedded in TaskDetails, so we ignore the separate output parameter
        self.add_task(id, details, handle).await;
    }

    #[instrument]
    pub async fn run_cleanup_task(&self, interval: SchedulerInterval) {
        let ttl = interval.into();

        // Step 1: Collect task states with minimal locking (read lock only)
        let task_states: Vec<(Uuid, TaskState)> = {
            let processes = self.processes.read().await;
            processes
                .iter()
                .map(|(uuid, state)| (*uuid, state.clone()))
                .collect()
        };
        // Lock released immediately after collecting

        // Step 2: Check each task without holding any locks
        let mut to_remove = vec![];
        for (uuid, state) in task_states {
            if let Some(handle) = &state.handle {
                // Check if handle is finished (async operation without locks)
                let is_finished = handle.read().await.is_finished();

                if is_finished {
                    // Check finish time (async operation without locks)
                    let details = state.details.read().await;
                    let should_remove = details.finish_time.is_some_and(|finish_time| {
                        chrono::Utc::now().signed_duration_since(finish_time) > ttl
                    });

                    if should_remove {
                        // Abort the handle (async operation without locks)
                        handle.write().await.abort();
                        to_remove.push(uuid);
                    }
                }
            }
        }

        // Step 3: Remove tasks with a write lock (minimal time)
        if !to_remove.is_empty() {
            let mut processes = self.processes.write().await;
            for uuid in to_remove {
                processes.remove(&uuid);
            }
            let active_count = processes.len();
            drop(processes); // Release lock before recording metrics

            metrics::metrics().record_task_cleanup(active_count);
        }
        // Write lock released immediately
    }
}

async fn spawn_process(
    cwd: &PathBuf,
    cmd: &str,
    args: &Vec<String>,
    env: &HashMap<String, String>,
    details: &Arc<RwLock<TaskDetails>>,
    _messenger: WebSocketMessenger,
) -> anyhow::Result<i32> {
    let mut child = Command::new(cmd)
        .args(args)
        .envs(env)
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start command");

    let stdout = child.stdout.take().expect("Failed to get stdout");
    let stderr = child.stderr.take().expect("Failed to get stderr");
    {
        let details = details.clone();
        let task_id = {
            let details_guard = details.read().await;
            details_guard.id
        };
        tokio::spawn(async move {
            read_unified_output(details.clone(), stdout, OutputStreamType::Stdout, task_id).await;
        });
    }

    {
        let details = details.clone();
        let task_id = {
            let details_guard = details.read().await;
            details_guard.id
        };
        tokio::spawn(async move {
            read_unified_output(details.clone(), stderr, OutputStreamType::Stderr, task_id).await;
        });
    }
    // Wait for the command to complete
    let status = child.wait().await?;

    let exit_code = status.code().unwrap_or_default();
    Ok(exit_code)
}

async fn read_unified_output(
    details: Arc<RwLock<TaskDetails>>,
    stream: impl AsyncRead + Unpin,
    stream_type: OutputStreamType,
    task_id: uuid::Uuid,
) {
    const BATCH_SIZE: usize = 20;
    const FLUSH_INTERVAL_MS: u64 = 100;

    let mut reader = BufReader::new(stream).lines();
    let mut buffer = TimedBuffer::new(BATCH_SIZE, FLUSH_INTERVAL_MS);

    while let Some(line_result) = reader.next_line().await.transpose() {
        match line_result {
            Ok(line) => {
                buffer.push((stream_type, line));

                if buffer.should_flush() {
                    add_output_lines(&details, buffer.flush(), task_id).await;
                }
            }
            Err(e) => {
                // Flush remaining buffer before error message
                if buffer.has_data() {
                    add_output_lines(&details, buffer.flush(), task_id).await;
                }
                add_output_line(
                    &details,
                    OutputStreamType::Stderr,
                    format!("Error reading output: {}", e),
                    task_id,
                )
                .await;
                break;
            }
        }
    }

    // Flush any remaining buffered lines when stream ends
    if buffer.has_data() {
        add_output_lines(&details, buffer.flush(), task_id).await;
    }
}
