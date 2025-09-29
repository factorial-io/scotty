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

/// Helper function to add a line to task output
async fn add_output_line(
    details: &Arc<RwLock<TaskDetails>>,
    stream_type: OutputStreamType,
    line: String,
    task_id: uuid::Uuid,
) {
    let mut details = details.write().await;
    details.output.add_line(stream_type, line);
    debug!("Added output line to task {}", task_id);
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

    pub async fn get_task_list(&self) -> Vec<TaskDetails> {
        let processes = self.processes.read().await;
        let mut task_list = Vec::new();
        for task_state in processes.values() {
            let details = task_state.details.read().await;
            task_list.push(details.clone());
        }
        task_list
    }

    pub async fn get_task_details(&self, uuid: &Uuid) -> Option<TaskDetails> {
        debug!("TaskManager: Getting task details for {}", uuid);
        let processes = self.processes.read().await;
        let task_state = processes.get(uuid)?;
        let details = task_state.details.read().await;
        Some(details.clone())
    }

    pub async fn get_task_output(&self, uuid: &Uuid) -> Option<TaskOutput> {
        debug!("TaskManager: Getting task output for {}", uuid);
        let processes = self.processes.read().await;
        let task_state = processes.get(uuid)?;
        let details = task_state.details.read().await;
        Some(details.output.clone())
    }

    /// Add a message to a task's output stream
    pub async fn add_task_message(
        &self,
        uuid: &Uuid,
        stream_type: OutputStreamType,
        message: String,
    ) -> bool {
        debug!("TaskManager: Adding message to task output for {}", uuid);
        let processes = self.processes.read().await;
        if let Some(task_state) = processes.get(uuid) {
            let mut details = task_state.details.write().await;
            details.output.add_line(stream_type, message);
            true
        } else {
            false
        }
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
        let mut details = details.write().await;
        details.last_exit_code = exit_code;
        details.finish_time = Some(chrono::Utc::now());
        details.state = state;
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
        let processes = self.processes.clone();
        let processes_lock = processes.write().await;

        let mut to_remove = vec![];

        for (uuid, state) in processes_lock.iter() {
            if let Some(handle) = &state.handle {
                let details = state.details.read().await;
                if handle.read().await.is_finished()
                    && details.finish_time.is_some_and(|finish_time| {
                        chrono::Utc::now().signed_duration_since(finish_time) > ttl
                    })
                {
                    handle.write().await.abort();
                    to_remove.push(*uuid);
                }
            }
        }

        // With the new self-contained streaming approach, no cleanup is needed
        drop(processes_lock); // Release lock before async operations

        // Remove tasks from the list
        let mut processes_lock = self.processes.write().await;
        for uuid in to_remove {
            processes_lock.remove(&uuid);
        }
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
    let mut reader = BufReader::new(stream).lines();

    while let Some(line_result) = reader.next_line().await.transpose() {
        match line_result {
            Ok(line) => {
                add_output_line(&details, stream_type, line, task_id).await;
            }
            Err(e) => {
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
}
