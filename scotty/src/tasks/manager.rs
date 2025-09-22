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

/// Helper function to add a line to output and broadcast it to subscribers
async fn add_and_broadcast_line(
    output: &Arc<RwLock<TaskOutput>>,
    stream_type: OutputStreamType,
    line: String,
    messenger: &WebSocketMessenger,
    task_id: uuid::Uuid,
) {
    let output_line = {
        let mut output = output.write().await;
        output.add_line(stream_type, line);
        output.lines.back().cloned()
    };

    if let Some(line) = output_line {
        use scotty_core::websocket::message::{TaskOutputData, WebSocketMessage};
        debug!("Broadcasting output update for task {}", task_id);

        let message = WebSocketMessage::TaskOutputData(TaskOutputData {
            task_id,
            lines: vec![line],
            is_historical: false,
            has_more: false,
        });

        messenger
            .broadcast_to_task_subscribers(task_id, message)
            .await;
    }
}

/// Helper function to handle task completion consistently
async fn handle_task_completion(
    details: &Arc<RwLock<TaskDetails>>,
    messenger: &WebSocketMessenger,
    task_id: uuid::Uuid,
    exit_code: Result<i32, anyhow::Error>,
) {
    debug!("Handling task completion for task {}", task_id);
    let (state, status_code, reason) = match exit_code {
        Ok(0) => (State::Finished, Some(0), "completed"),
        Ok(e) => (State::Failed, Some(e), "failed"),
        Err(_) => (State::Failed, None, "failed"),
    };

    TaskManager::set_task_finished(details, status_code, state).await;

    use scotty_core::websocket::message::WebSocketMessage;
    debug!("Broadcasting completion for task {}", task_id);

    let message = WebSocketMessage::TaskOutputStreamEnded {
        task_id,
        reason: reason.to_string(),
    };

    messenger
        .broadcast_to_task_subscribers(task_id, message)
        .await;
}

#[derive(Clone, Debug)]
pub struct TaskManager {
    processes: Arc<RwLock<HashMap<Uuid, TaskState>>>,
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
        let output = task_state.output.read().await;
        Some(output.clone())
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
            let mut output = task_state.output.write().await;
            output.add_line(stream_type, message);
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
        output_settings: &OutputSettings,
    ) -> Uuid {
        let cwd = cwd.to_path_buf();
        let cmd = cmd.to_string();
        let args = args.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        let env = env.clone();
        let id = details.read().await.id;

        // Create TaskOutput for this task with settings
        let output = Arc::new(RwLock::new(TaskOutput::new_with_settings(
            id,
            output_settings,
        )));

        {
            debug!("details write lock, setting state to running");
            let mut details = details.write().await;
            details.state = State::Running;
        }

        let handle = {
            let details = details.clone();
            let output = output.clone();
            let messenger = self.messenger.clone();

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
                    &output,
                    messenger_for_spawn.clone(),
                )
                .await;

                handle_task_completion(&details, &messenger, id, exit_result).await;
            })
        };
        {
            self.add_task_with_output(
                &id,
                details.clone(),
                Some(Arc::new(RwLock::new(handle))),
                output.clone(),
            )
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
        let output = Arc::new(RwLock::new(TaskOutput::new(*id)));
        self.add_task_with_output(id, details, handle, output).await;
    }

    pub async fn add_task_with_output(
        &self,
        id: &Uuid,
        details: Arc<RwLock<TaskDetails>>,
        handle: Option<Arc<RwLock<tokio::task::JoinHandle<()>>>>,
        output: Arc<RwLock<TaskOutput>>,
    ) {
        let mut processes = self.processes.write().await;
        let task_state = TaskState {
            details,
            handle,
            output,
        };
        processes.insert(*id, task_state);
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

        // Clean up WebSocket subscriptions before removing tasks
        drop(processes_lock); // Release lock before async operations
        for uuid in &to_remove {
            self.messenger.cleanup_task_subscriptions(*uuid).await;
        }

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
    _details: &Arc<RwLock<TaskDetails>>,
    output: &Arc<RwLock<TaskOutput>>,
    messenger: WebSocketMessenger,
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
        let output = output.clone();
        let messenger_ref = messenger.clone();
        let task_id = {
            let output_guard = output.read().await;
            output_guard.task_id
        };
        tokio::spawn(async move {
            read_unified_output(
                output.clone(),
                stdout,
                OutputStreamType::Stdout,
                messenger_ref,
                task_id,
            )
            .await;
        });
    }

    {
        let output = output.clone();
        let messenger_ref = messenger.clone();
        let task_id = {
            let output_guard = output.read().await;
            output_guard.task_id
        };
        tokio::spawn(async move {
            read_unified_output(
                output.clone(),
                stderr,
                OutputStreamType::Stderr,
                messenger_ref,
                task_id,
            )
            .await;
        });
    }
    // Wait for the command to complete
    let status = child.wait().await?;

    let exit_code = status.code().unwrap_or_default();
    Ok(exit_code)
}

async fn read_unified_output(
    output: Arc<RwLock<TaskOutput>>,
    stream: impl AsyncRead + Unpin,
    stream_type: OutputStreamType,
    messenger: WebSocketMessenger,
    task_id: uuid::Uuid,
) {
    let mut reader = BufReader::new(stream).lines();

    while let Some(line_result) = reader.next_line().await.transpose() {
        match line_result {
            Ok(line) => {
                add_and_broadcast_line(&output, stream_type, line, &messenger, task_id).await;
            }
            Err(e) => {
                add_and_broadcast_line(
                    &output,
                    OutputStreamType::Stderr,
                    format!("Error reading output: {}", e),
                    &messenger,
                    task_id,
                )
                .await;
                break;
            }
        }
    }
}
