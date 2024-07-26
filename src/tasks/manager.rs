#![allow(dead_code)]

use std::path::PathBuf;
use std::{collections::HashMap, sync::Arc, time::SystemTime};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};

use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::instrument;
use utoipa::ToResponse;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct TaskManager {
    processes: Arc<RwLock<HashMap<Uuid, TaskState>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum State {
    Running,
    Finished,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToResponse, utoipa::ToSchema)]
pub struct TaskDetails {
    pub id: Uuid,
    pub command: String,
    pub state: State,
    pub output: String,
    pub start_time: SystemTime,
    pub finish_time: Option<SystemTime>,
    pub last_read_position: usize,
}

impl Default for TaskDetails {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            command: "".to_string(),
            state: State::Running,
            output: "".to_string(),
            start_time: SystemTime::now(),
            finish_time: None,
            last_read_position: 0,
        }
    }
}

#[derive(Debug)]
struct TaskState {
    handle: Option<tokio::task::JoinHandle<()>>,
    details: Arc<RwLock<TaskDetails>>,
}

impl TaskManager {
    pub fn new() -> Self {
        let manager = Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
        };

        manager
    }

    pub async fn get_task_details(&self, uuid: &Uuid) -> Option<TaskDetails> {
        let processes = self.processes.read().await;
        let task_state = processes.get(uuid);
        if task_state.is_none() {
            return None;
        }
        let task_state = task_state.unwrap();
        let details = task_state.details.read().await;
        Some(details.clone())
    }

    pub async fn start_process(&self, cwd: &PathBuf, cmd: &str, args: &[&str]) -> Uuid {
        let details = Arc::new(RwLock::new(TaskDetails::default()));

        let cwd = cwd.clone();
        let cmd = cmd.to_string();
        let args = args.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        let id = details.read().await.id;

        {
            let mut details = details.write().await;
            details.command = cmd.to_string();
            details.state = State::Running;
        }

        let handle = {
            let details = details.clone();
            let handle = tokio::task::spawn(async move {
                let details = details.clone();
                let exit_code = spawn_process(&cwd, &cmd, &args, &details).await;
                match exit_code {
                    Ok(0) => {
                        let mut details = details.write().await;
                        details.state = State::Finished;
                        details.finish_time = Some(SystemTime::now());
                    }
                    Ok(_) => {
                        let mut details = details.write().await;
                        details.finish_time = Some(SystemTime::now());
                        details.state = State::Failed;
                    }
                    Err(e) => {
                        let mut details = details.write().await;
                        details.finish_time = Some(SystemTime::now());
                        details.state = State::Failed;
                        details.output = e.to_string();
                    }
                }
            });
            handle
        };
        {
            let details = details.clone();
            self.processes.write().await.insert(
                id,
                TaskState {
                    handle: Some(handle),
                    details: details.clone(),
                },
            );
        }

        id
    }

    #[instrument]
    pub async fn run_cleanup_task(&self) {
        let processes = self.processes.clone();
        let mut processes_lock = processes.write().await;
        processes_lock.retain(|_, state| {
            if let Some(handle) = &state.handle {
                if handle.is_finished() {
                    state.handle.as_ref().unwrap().abort();
                    return false;
                }
            }
            true
        });
    }
}

async fn spawn_process(
    cwd: &PathBuf,
    cmd: &str,
    args: &Vec<String>,
    details: &Arc<RwLock<TaskDetails>>,
) -> anyhow::Result<i32> {
    let mut child = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start command");

    // Ensure we can get stdout
    let stdout = child.stdout.take().expect("Failed to get stdout");

    // Wrap stdout in a BufReader
    let mut reader = BufReader::new(stdout).lines();

    let mut output = String::new();
    // Stream the stdout into the variable
    while let Some(line) = reader.next_line().await? {
        output.push_str(&line);
        output.push('\n');
        {
            let mut details = details.write().await;
            details.output = output.to_string();
        }
    }
    // Wait for the command to complete
    let status = child.wait().await?;

    let exit_code = status.code().unwrap_or_default();
    Ok(exit_code)
}
