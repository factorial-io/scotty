#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::{collections::HashMap, sync::Arc};

use scotty_core::settings::{
    output::OutputSettings,
    scheduler_interval::SchedulerInterval,
};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};

use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{info, instrument};
use uuid::Uuid;

use scotty_core::tasks::task_details::{State, TaskDetails, TaskState};
use scotty_core::output::{TaskOutput, OutputStreamType};

#[derive(Clone, Debug)]
pub struct TaskManager {
    processes: Arc<RwLock<HashMap<Uuid, TaskState>>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
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
        let processes = self.processes.read().await;
        let task_state = processes.get(uuid);
        task_state?;
        let task_state = task_state.unwrap();
        let details = task_state.details.read().await;
        Some(details.clone())
    }

    pub async fn get_task_output(&self, uuid: &Uuid) -> Option<TaskOutput> {
        let processes = self.processes.read().await;
        let task_state = processes.get(uuid)?;
        let output = task_state.output.read().await;
        Some(output.clone())
    }

    /// Add a message to a task's output stream
    pub async fn add_task_message(&self, uuid: &Uuid, stream_type: OutputStreamType, message: String) -> bool {
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
        self.add_task_message(uuid, OutputStreamType::Stdout, message).await
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

    pub async fn start_process(
        &self,
        cwd: &Path,
        cmd: &str,
        args: &[&str],
        env: &HashMap<String, String>,
        details: Arc<RwLock<TaskDetails>>,
    ) -> Uuid {
        self.start_process_with_settings(cwd, cmd, args, env, details, &OutputSettings::default()).await
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
        let output = Arc::new(RwLock::new(TaskOutput::new_with_settings(id, output_settings)));

        {
            let mut details = details.write().await;
            details.state = State::Running;
        }

        let handle = {
            let details = details.clone();
            let output = output.clone();

            tokio::task::spawn(async move {
                info!(
                    "Starting process with uuid {} in folder {}: {:?} {}",
                    &id,
                    cwd.display(),
                    cmd,
                    args.join(" ")
                );
                let details = details.clone();
                let exit_code = spawn_process(&cwd, &cmd, &args, &env, &details, &output).await;

                match exit_code {
                    Ok(0) => {
                        let mut details = details.write().await;
                        details.last_exit_code = Some(0);
                        details.finish_time = Some(chrono::Utc::now());
                    }
                    Ok(e) => {
                        let mut details = details.write().await;
                        details.finish_time = Some(chrono::Utc::now());
                        details.last_exit_code = Some(e);
                        details.state = State::Failed;
                    }
                    Err(_e) => {
                        let mut details = details.write().await;
                        details.finish_time = Some(chrono::Utc::now());
                        details.state = State::Failed;
                        // Error message will be added to output separately
                    }
                }
            })
        };
        {
            self.add_task_with_output(&id, details.clone(), Some(Arc::new(RwLock::new(handle))), output.clone())
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
        let task_state = TaskState { details, handle, output };
        processes.insert(*id, task_state);
    }

    #[instrument]
    pub async fn run_cleanup_task(&self, interval: SchedulerInterval) {
        let ttl = interval.into();
        let processes = self.processes.clone();
        let mut processes_lock = processes.write().await;

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
        tokio::spawn(async move {
            read_unified_output(output.clone(), stdout, OutputStreamType::Stdout).await;
        });
    }

    {
        let output = output.clone();
        tokio::spawn(async move {
            read_unified_output(output.clone(), stderr, OutputStreamType::Stderr).await;
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
) {
    let mut reader = BufReader::new(stream).lines();

    while let Some(line_result) = reader.next_line().await.transpose() {
        match line_result {
            Ok(line) => {
                let mut output = output.write().await;
                output.add_line(stream_type, line);
            }
            Err(e) => {
                let mut output = output.write().await;
                output.add_line(OutputStreamType::Stderr, format!("Error reading output: {}", e));
                break;
            }
        }
    }
}
