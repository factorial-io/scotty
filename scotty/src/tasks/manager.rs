#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::{collections::HashMap, sync::Arc};

use scotty_core::settings::scheduler_interval::SchedulerInterval;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};

use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{info, instrument};
use uuid::Uuid;

use scotty_core::tasks::task_details::{State, TaskDetails, TaskState};

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
        let cwd = cwd.to_path_buf();
        let cmd = cmd.to_string();
        let args = args.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        let env = env.clone();
        let id = details.read().await.id;

        {
            let mut details = details.write().await;
            details.state = State::Running;
        }

        let handle = {
            let details = details.clone();

            tokio::task::spawn(async move {
                info!(
                    "Starting process with uuid {} in folder {}: {:?} {}",
                    &id,
                    cwd.display(),
                    cmd,
                    args.join(" ")
                );
                let details = details.clone();
                let exit_code = spawn_process(&cwd, &cmd, &args, &env, &details).await;

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
                    Err(e) => {
                        let mut details = details.write().await;
                        details.finish_time = Some(chrono::Utc::now());
                        details.state = State::Failed;
                        details.stdout = e.to_string();
                    }
                }
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
                    && details.finish_time.map_or(false, |finish_time| {
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
    details: &Arc<RwLock<TaskDetails>>,
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
        tokio::spawn(async move {
            read_output(details.clone(), stdout, true).await;
        });
    }

    {
        let details = details.clone();
        tokio::spawn(async move {
            read_output(details.clone(), stderr, false).await;
        });
    }
    // Wait for the command to complete
    let status = child.wait().await?;

    let exit_code = status.code().unwrap_or_default();
    Ok(exit_code)
}

async fn read_output(
    details: Arc<RwLock<TaskDetails>>,
    out: impl AsyncRead + Unpin,
    is_stdout: bool,
) {
    // Wrap stdout in a BufReader
    let mut reader = BufReader::new(out).lines();

    while let Some(line) = reader.next_line().await.unwrap() {
        let mut details = details.write().await;
        let output = match is_stdout {
            true => &mut details.stdout,
            false => &mut details.stderr,
        };
        output.push_str(&line);
        output.push('\n');
    }
}
