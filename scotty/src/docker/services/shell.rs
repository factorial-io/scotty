use bollard::exec::{CreateExecOptions, StartExecOptions, StartExecResults};
use bollard::Docker;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, RwLock};
use tokio::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::app_state::SharedAppState;
use scotty_core::apps::app_data::AppData;
use scotty_core::settings::shell::ShellSettings;
use scotty_core::websocket::message::WebSocketMessage;
use scotty_types::{
    ShellDataType, ShellSessionData, ShellSessionEnd, ShellSessionError, ShellSessionInfo,
    ShellSessionRequest,
};

use thiserror::Error;

/// Error types for shell session operations
#[derive(Error, Debug, Clone, utoipa::ToSchema)]
pub enum ShellServiceError {
    #[error("Service '{service}' not found in app '{app}'")]
    ServiceNotFound { service: String, app: String },

    #[error("Service '{service}' has no container ID")]
    NoContainerId { service: String },

    #[error("Session '{session_id}' not found")]
    SessionNotFound { session_id: Uuid },

    #[error("Maximum sessions per app ({limit}) reached")]
    MaxSessionsPerApp { limit: usize },

    #[error("Maximum global sessions ({limit}) reached")]
    MaxSessionsGlobal { limit: usize },

    #[error("Failed to send command to session: {reason}")]
    CommandSendFailed { reason: String },

    #[error("Docker operation failed: {operation} - {message}")]
    DockerOperationFailed { operation: String, message: String },
}

/// Result type alias for shell service operations
pub type ShellServiceResult<T> = Result<T, ShellServiceError>;

/// Active shell sessions tracked by the service
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ShellSession {
    pub session_id: Uuid,
    pub app_name: String,
    pub service_name: String,
    pub container_id: String,
    pub exec_id: String,
    /// Control commands (resize, terminate), handled by the session loop
    pub sender: mpsc::Sender<ShellCommand>,
    /// Stdin commands (input, close), handled by a separate writer task so a
    /// process that is slow to read stdin never stops the output from flowing
    pub stdin_sender: mpsc::Sender<ShellCommand>,
    pub client_id: Uuid, // WebSocket client that owns this session
    /// Whether the exec runs with a tty
    pub tty: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl ShellSession {
    /// Check if session has expired based on TTL
    #[allow(dead_code)]
    pub fn is_expired(&self, ttl: std::time::Duration) -> bool {
        let age = chrono::Utc::now() - self.created_at;
        age.num_seconds() as u64 > ttl.as_secs()
    }

    /// Convert to ShellSessionInfo for WebSocket messages
    #[allow(dead_code)]
    pub fn to_info(&self, shell_command: String) -> ShellSessionInfo {
        ShellSessionInfo::new(
            self.session_id,
            self.app_name.clone(),
            self.service_name.clone(),
            self.container_id.clone(),
            shell_command,
        )
    }

    /// Send a command to this session
    pub async fn send_command(&self, cmd: ShellCommand) -> ShellServiceResult<()> {
        let sender = match cmd {
            ShellCommand::Input(_) | ShellCommand::CloseStdin => &self.stdin_sender,
            ShellCommand::Resize { .. } | ShellCommand::Terminate => &self.sender,
        };
        sender
            .send(cmd)
            .await
            .map_err(|e| ShellServiceError::CommandSendFailed {
                reason: e.to_string(),
            })
    }
}

/// Commands that can be sent to a shell session
#[derive(Debug)]
pub enum ShellCommand {
    Input(Vec<u8>),
    Resize {
        width: u16,
        height: u16,
    },
    /// Close the exec's stdin so the process reads end-of-file
    CloseStdin,
    Terminate,
}

/// Guard to ensure session cleanup even on panic
///
/// This guard uses RAII to guarantee that a session is removed from the active
/// sessions map when it goes out of scope, even if the task panics.
struct SessionGuard {
    session_id: Uuid,
    sessions: Arc<RwLock<HashMap<Uuid, ShellSession>>>,
}

impl SessionGuard {
    fn new(session_id: Uuid, sessions: Arc<RwLock<HashMap<Uuid, ShellSession>>>) -> Self {
        Self {
            session_id,
            sessions,
        }
    }
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        // Ensure session cleanup even on panic
        // We need to block on the async operation since Drop is synchronous
        let session_id = self.session_id;
        let sessions = self.sessions.clone();

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let mut sessions = sessions.write().await;
                sessions.remove(&session_id);
                info!("SessionGuard: cleaned up session {}", session_id);
            })
        });
    }
}

/// How a shell session ended
#[derive(Debug)]
enum SessionOutcome {
    /// The process exited, with its exit code if Docker reported one
    Exited(Option<i32>),
    /// The session reached its TTL
    Timeout,
    /// The client asked to terminate the session
    Terminated,
    /// Reading output or writing input failed
    Failed(String),
}

/// What a running session needs to report about itself
struct SessionContext {
    session_id: Uuid,
    client_id: Uuid,
    app_name: String,
    service_name: String,
    started: tokio::time::Instant,
    app_state: SharedAppState,
}

impl SessionContext {
    /// Send a message to the client that owns the session
    async fn send(&self, message: WebSocketMessage) -> Result<(), String> {
        self.app_state
            .messenger
            .send_to_client(self.client_id, message)
            .await
            .map_err(|e| e.to_string())
    }

    /// Record metrics and the audit log for the end of the session and tell
    /// the client about it.
    async fn end(&self, outcome: SessionOutcome) {
        let duration_secs = self.started.elapsed().as_secs_f64();
        let (reason, exit_code) = match &outcome {
            SessionOutcome::Exited(code) => {
                crate::metrics::shell::record_session_ended(duration_secs);
                ("Shell exited".to_string(), *code)
            }
            SessionOutcome::Timeout => {
                crate::metrics::shell::record_session_timeout(duration_secs);
                ("Session timeout".to_string(), None)
            }
            SessionOutcome::Terminated => {
                crate::metrics::shell::record_session_ended(duration_secs);
                ("Terminated by user".to_string(), None)
            }
            SessionOutcome::Failed(error) => {
                crate::metrics::shell::record_session_error(duration_secs);
                (error.clone(), None)
            }
        };

        info!(
            event = "shell_session_ended",
            session_id = %self.session_id,
            client_id = %self.client_id,
            app_name = %self.app_name,
            service_name = %self.service_name,
            duration_seconds = duration_secs,
            reason = %reason,
            exit_code = ?exit_code,
            "Shell session ended for compliance audit"
        );

        let message = match outcome {
            SessionOutcome::Failed(error) => {
                WebSocketMessage::ShellSessionError(ShellSessionError {
                    session_id: self.session_id,
                    error,
                })
            }
            _ => WebSocketMessage::ShellSessionEnded(ShellSessionEnd {
                session_id: self.session_id,
                exit_code,
                reason,
            }),
        };
        if let Err(e) = self.send(message).await {
            warn!(
                "Failed to send end of shell session {} to client {}: {}",
                self.session_id, self.client_id, e
            );
        }
    }
}

/// How often to poll Docker for the exit code once the output stream of an
/// exec has closed, and how long at most. Docker may still report the exec
/// as running for a moment after its streams are closed.
const EXIT_CODE_POLL_INTERVAL: Duration = Duration::from_millis(50);
const EXIT_CODE_TIMEOUT: Duration = Duration::from_secs(3);

/// Read the exit code of a finished exec, waiting at most
/// [`EXIT_CODE_TIMEOUT`] for Docker to record it.
async fn wait_for_exit_code(docker: &Docker, exec_id: &str) -> Option<i32> {
    let poll = async {
        loop {
            match docker.inspect_exec(exec_id).await {
                Ok(info) if info.running == Some(false) => {
                    return info.exit_code.map(|c| c as i32);
                }
                Ok(_) => tokio::time::sleep(EXIT_CODE_POLL_INTERVAL).await,
                Err(e) => {
                    warn!("Failed to inspect exec {}: {}", exec_id, e);
                    return None;
                }
            }
        }
    };
    match tokio::time::timeout(EXIT_CODE_TIMEOUT, poll).await {
        Ok(exit_code) => exit_code,
        Err(_) => {
            warn!(
                "Exec {} still reported as running {:?} after its output closed",
                exec_id, EXIT_CODE_TIMEOUT
            );
            None
        }
    }
}

/// Number of stdin chunks queued per session before the client is slowed down
const STDIN_QUEUE_CAPACITY: usize = 32;

/// Shell output waits while the client has this many messages queued. Well
/// below the per-client buffer (1000 messages), so output never overflows it
/// and other messages (logs, task updates) still have room.
const OUTPUT_QUEUE_HIGH_WATER: usize = 256;

/// How often held-back output checks whether the client caught up.
const OUTPUT_RETRY_INTERVAL: Duration = Duration::from_millis(5);

/// Write stdin commands to the exec's input until the queue closes.
///
/// Runs in its own task: a process that writes a lot of output before it
/// reads stdin would otherwise block the input write, the session loop would
/// stop reading output, and both pipes would fill up (deadlock). Returns the
/// write error, if any, so the session loop can end the session.
async fn write_stdin(
    session_id: Uuid,
    mut input: std::pin::Pin<Box<dyn tokio::io::AsyncWrite + Send>>,
    mut commands: mpsc::Receiver<ShellCommand>,
) -> Result<(), std::io::Error> {
    let mut closed = false;
    while let Some(cmd) = commands.recv().await {
        match cmd {
            ShellCommand::Input(data) if closed => {
                warn!(
                    "Dropping {} bytes of input for shell session {}: stdin is closed",
                    data.len(),
                    session_id
                );
            }
            ShellCommand::Input(data) => {
                input.write_all(&data).await?;
                input.flush().await?;
            }
            ShellCommand::CloseStdin if !closed => {
                closed = true;
                info!("Closing stdin of shell session {}", session_id);
                if let Err(e) = input.shutdown().await {
                    warn!(
                        "Failed to close stdin of shell session {}: {}",
                        session_id, e
                    );
                }
            }
            ShellCommand::CloseStdin => {}
            // `ShellSession::send_command` routes these to the session loop.
            ShellCommand::Resize { .. } | ShellCommand::Terminate => {
                warn!(
                    "Ignoring control command {:?} sent to the stdin of shell session {}",
                    cmd, session_id
                );
            }
        }
    }
    Ok(())
}

/// Environment variable that tags every process of a shell session
const SESSION_ENV_VAR: &str = "SCOTTY_SHELL_SESSION";

/// Exit code of the stop script when the container lacks a tool it needs
const STOP_SCRIPT_MISSING_TOOLS: i64 = 127;

/// Stop every process of a shell session in its container.
///
/// Docker does not kill an exec's process when the attached connection
/// closes. With a tty the process gets a hang-up; without one it keeps
/// running (e.g. `tail -f` after the client pressed Ctrl-C). There is no
/// Docker API to kill an exec, so a second exec sends TERM, then KILL, to
/// all processes tagged with the session's environment variable.
///
/// This needs `sh`, `tr`, `grep` and `kill` in the container and read
/// access to `/proc/<pid>/environ` of the session's processes. When they are
/// missing (e.g. distroless images), a warning is logged and the processes
/// keep running until they exit by themselves.
async fn stop_session_processes(docker: &Docker, container_id: &str, session_id: Uuid) {
    let script = format!(
        "for t in tr grep kill; do command -v $t >/dev/null 2>&1 || exit {missing}; done; \
         m={var}={id}; \
         for sig in TERM KILL; do \
           for d in /proc/[0-9]*; do \
             [ \"$d\" = /proc/$$ ] && continue; \
             tr '\\0' '\\n' < \"$d/environ\" 2>/dev/null | grep -qx \"$m\" \
               && kill -$sig \"${{d#/proc/}}\" 2>/dev/null; \
           done; \
           [ $sig = TERM ] && sleep 2; \
         done; true",
        missing = STOP_SCRIPT_MISSING_TOOLS,
        var = SESSION_ENV_VAR,
        id = session_id
    );
    let options = CreateExecOptions {
        cmd: Some(vec!["sh", "-c", script.as_str()]),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        ..Default::default()
    };
    let exec_id = match docker.create_exec(container_id, options).await {
        Ok(exec) => exec.id,
        Err(e) => {
            warn!(
                "Failed to stop processes of shell session {}: {}",
                session_id, e
            );
            return;
        }
    };
    // Run attached and wait for the script, so its result can be reported.
    match docker.start_exec(&exec_id, None).await {
        Ok(StartExecResults::Attached { mut output, .. }) => while output.next().await.is_some() {},
        Ok(StartExecResults::Detached) => {}
        Err(e) => {
            warn!(
                "Failed to stop processes of shell session {}: {}",
                session_id, e
            );
            return;
        }
    }
    match wait_for_exit_code(docker, &exec_id).await {
        Some(0) => info!("Stopped processes of shell session {}", session_id),
        Some(code) if i64::from(code) == STOP_SCRIPT_MISSING_TOOLS => warn!(
            "Could not stop processes of shell session {}: the container lacks sh, tr, grep \
             or kill; they keep running until they exit",
            session_id
        ),
        code => warn!(
            "Stopping processes of shell session {} failed (exit code {:?}); \
             they may still be running",
            session_id, code
        ),
    }
}

/// Drives a started exec: forwards its output to the client and the
/// client's commands to the exec until the session ends.
struct SessionRunner {
    ctx: SessionContext,
    docker: Docker,
    exec_id: String,
    container_id: String,
    tty: bool,
    ttl: std::time::Duration,
}

/// What to do after reading one item of the exec's output
enum OutputStep {
    /// Keep going
    Continue,
    /// The client is behind; send this data once it caught up
    Hold(ShellSessionData),
    /// The session is over
    End(Option<SessionOutcome>),
}

impl SessionRunner {
    /// Run the session to its end. Returns whether the session's processes
    /// may still be running and have to be stopped.
    async fn run(
        &self,
        exec_stream: StartExecResults,
        mut control: mpsc::Receiver<ShellCommand>,
        stdin: mpsc::Receiver<ShellCommand>,
    ) -> bool {
        let session_id = self.ctx.session_id;
        let client_id = self.ctx.client_id;
        info!(
            "Starting shell session {} for container {}",
            session_id, self.container_id
        );

        let (input, mut output) = match exec_stream {
            StartExecResults::Attached { input, output } => (input, output),
            StartExecResults::Detached => {
                error!(
                    "Shell session {} started in detached mode (unexpected)",
                    session_id
                );
                self.ctx
                    .end(SessionOutcome::Failed(
                        "Shell started in detached mode".to_string(),
                    ))
                    .await;
                return false;
            }
        };

        let mut status_check_interval = tokio::time::interval(Duration::from_millis(500));
        status_check_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut stdin_writer = tokio::spawn(write_stdin(session_id, input, stdin));
        let mut stdin_writer_done = false;
        // Output held back until the client catches up. While set, no more
        // output is read from Docker, which slows down the process instead of
        // dropping data.
        let mut held_output: Option<ShellSessionData> = None;
        let mut output_retry = tokio::time::interval(OUTPUT_RETRY_INTERVAL);
        output_retry.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let deadline = self.ctx.started + self.ttl;

        // `None`: the session ended without a message to the client (the
        // client is gone). Otherwise, how it ended.
        let outcome: Option<SessionOutcome> = loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => {
                    info!("Shell session {} expired after TTL", session_id);
                    break Some(SessionOutcome::Timeout);
                }

                // Periodically check if exec is still running. Only for TTY
                // mode, where a background process can keep the pty (and
                // therefore the output stream) open after the shell exited.
                // Without a TTY the output stream closes when the process
                // exits, after all output was delivered.
                _ = status_check_interval.tick(), if self.tty => {
                    match self.docker.inspect_exec(&self.exec_id).await {
                        Ok(exec_info) if exec_info.running == Some(false) => {
                            info!("Shell session {} detected exec completion via status check", session_id);
                            break Some(SessionOutcome::Exited(exec_info.exit_code.map(|c| c as i32)));
                        }
                        Ok(_) => {}
                        Err(e) => {
                            warn!("Failed to inspect exec {} for session {}: {}", self.exec_id, session_id, e);
                        }
                    }
                }

                // The stdin writer stopped: report a write failure. A clean
                // stop (queue closed) needs no action.
                result = &mut stdin_writer, if !stdin_writer_done => {
                    stdin_writer_done = true;
                    let error = match result {
                        Ok(Ok(())) => continue,
                        Ok(Err(e)) => e.to_string(),
                        Err(e) => format!("stdin writer failed: {}", e),
                    };
                    error!("Failed to write to shell {}: {}", session_id, error);
                    break Some(SessionOutcome::Failed(format!("Failed to write input: {}", error)));
                }

                Some(cmd) = control.recv() => {
                    match cmd {
                        ShellCommand::Resize { width, height } => {
                            if let Err(e) = self.docker.resize_exec(
                                &self.exec_id,
                                bollard::exec::ResizeExecOptions { height, width },
                            ).await {
                                warn!("Failed to resize TTY for session {}: {}", session_id, e);
                            }
                        }
                        ShellCommand::Terminate => {
                            info!("Terminating shell session {} by request", session_id);
                            break Some(SessionOutcome::Terminated);
                        }
                        // `ShellSession::send_command` routes these to the stdin writer.
                        ShellCommand::Input(_) | ShellCommand::CloseStdin => {
                            warn!("Ignoring stdin command on the control channel of shell session {}", session_id);
                        }
                    }
                }

                // Send held-back output once the client caught up
                _ = output_retry.tick(), if held_output.is_some() => {
                    match self.ctx.app_state.messenger.queued_messages(client_id).await {
                        Some(queued) if queued >= OUTPUT_QUEUE_HIGH_WATER => {}
                        Some(_) => {
                            let data = held_output.take().expect("checked by the guard");
                            if let Err(e) = self.ctx.send(WebSocketMessage::ShellSessionData(data)).await {
                                warn!("Failed to send output to client {}: {} - terminating session", client_id, e);
                                break None;
                            }
                        }
                        None => {
                            warn!("Client {} is gone - terminating shell session {}", client_id, session_id);
                            break None;
                        }
                    }
                }

                result = output.next(), if held_output.is_none() => {
                    match self.handle_output(result).await {
                        OutputStep::Continue => {}
                        OutputStep::Hold(data) => held_output = Some(data),
                        OutputStep::End(outcome) => break outcome,
                    }
                }
            }
        };

        // The session is over; stop writing to the exec.
        stdin_writer.abort();
        let process_exited = matches!(outcome, Some(SessionOutcome::Exited(_)));
        if let Some(outcome) = outcome {
            self.ctx.end(outcome).await;
        } else {
            // The client is gone; record the end for metrics and the audit log.
            let duration_secs = self.ctx.started.elapsed().as_secs_f64();
            crate::metrics::shell::record_session_ended(duration_secs);
            info!(
                event = "shell_session_ended",
                session_id = %session_id,
                client_id = %client_id,
                app_name = %self.ctx.app_name,
                service_name = %self.ctx.service_name,
                duration_seconds = duration_secs,
                reason = "Client gone",
                "Shell session ended for compliance audit"
            );
        }
        // Without a tty nothing hangs up the process when the connection
        // closes, so it has to be stopped explicitly.
        !self.tty && !process_exited
    }

    /// Forward one item of the exec's output to the client
    async fn handle_output(
        &self,
        item: Option<Result<bollard::container::LogOutput, bollard::errors::Error>>,
    ) -> OutputStep {
        use bollard::container::LogOutput;
        let session_id = self.ctx.session_id;
        let client_id = self.ctx.client_id;

        let log_output = match item {
            Some(Ok(log_output)) => log_output,
            Some(Err(e)) => {
                error!("Failed to read from shell stream {}: {}", session_id, e);
                return OutputStep::End(Some(SessionOutcome::Failed(format!(
                    "Failed to read output: {}",
                    e
                ))));
            }
            // Stream closed: the process exited and all of its output has
            // been forwarded.
            None => {
                info!("Shell session {} ended (stream closed)", session_id);
                // Blocks the session loop for at most EXIT_CODE_TIMEOUT; with
                // the output closed there is nothing left to forward, and
                // control commands can wait.
                let exit_code = wait_for_exit_code(&self.docker, &self.exec_id).await;
                return OutputStep::End(Some(SessionOutcome::Exited(exit_code)));
            }
        };

        let (data_type, message) = match log_output {
            LogOutput::StdOut { message } => (ShellDataType::Output, message),
            LogOutput::StdErr { message } => (ShellDataType::Stderr, message),
            // TTY mode sends output as Console
            LogOutput::Console { message } => (ShellDataType::Output, message),
            LogOutput::StdIn { .. } => return OutputStep::Continue,
        };
        // With a TTY, keep plain text for existing clients. Without one, send
        // the exact bytes.
        let data = if self.tty {
            ShellSessionData::text(session_id, data_type, String::from_utf8_lossy(&message))
        } else {
            ShellSessionData::binary(session_id, data_type, &message)
        };

        // Hold the output back while the client is behind. The check and the
        // send below are not atomic: other tasks may queue messages for the
        // client in between. That is fine, the high-water mark leaves plenty
        // of room below the per-client buffer.
        let queued = self
            .ctx
            .app_state
            .messenger
            .queued_messages(client_id)
            .await;
        if queued.is_some_and(|q| q >= OUTPUT_QUEUE_HIGH_WATER) {
            return OutputStep::Hold(data);
        }

        // If we can't send output, client is gone - terminate session
        if let Err(e) = self
            .ctx
            .send(WebSocketMessage::ShellSessionData(data))
            .await
        {
            warn!(
                "Failed to send output to client {}: {} - terminating session",
                client_id, e
            );
            return OutputStep::End(None);
        }
        OutputStep::Continue
    }
}

/// Service for managing container shell sessions
#[derive(Debug, Clone)]
pub struct ShellService {
    docker: Docker,
    active_sessions: Arc<RwLock<HashMap<Uuid, ShellSession>>>,
    shell_settings: ShellSettings,
}

impl ShellService {
    pub fn new(docker: Docker, shell_settings: ShellSettings) -> Self {
        Self {
            docker,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            shell_settings,
        }
    }

    /// Create a new shell session for `request` in `app_data`, owned by the
    /// WebSocket client `client_id`.
    pub async fn create_session(
        &self,
        app_state: &SharedAppState,
        app_data: &AppData,
        request: &ShellSessionRequest,
        client_id: Uuid,
    ) -> ShellServiceResult<Uuid> {
        let service_name = request.service_name.as_str();
        let tty = request.tty;

        // Check session limits
        {
            let sessions = self.active_sessions.read().await;
            let app_sessions = sessions
                .values()
                .filter(|s| s.app_name == app_data.name)
                .count();

            if app_sessions >= self.shell_settings.max_sessions_per_app {
                return Err(ShellServiceError::MaxSessionsPerApp {
                    limit: self.shell_settings.max_sessions_per_app,
                });
            }

            if sessions.len() >= self.shell_settings.max_sessions_global {
                return Err(ShellServiceError::MaxSessionsGlobal {
                    limit: self.shell_settings.max_sessions_global,
                });
            }
        }

        // Find the container for the service
        let container_id = app_data
            .get_container_id_for_service(service_name)
            .ok_or_else(|| {
                if app_data.find_container_by_service(service_name).is_some() {
                    ShellServiceError::NoContainerId {
                        service: service_name.to_string(),
                    }
                } else {
                    ShellServiceError::ServiceNotFound {
                        service: service_name.to_string(),
                        app: app_data.name.clone(),
                    }
                }
            })?
            .clone();

        // Generate session ID
        let session_id = Uuid::new_v4();

        // Determine shell command to use
        let shell_cmd = request
            .shell_command
            .clone()
            .unwrap_or_else(|| self.shell_settings.default_shell.clone());

        // Prepare environment variables
        // Tag the session's processes so they can be found and stopped later;
        // child processes inherit the variable.
        let env_vars: Vec<String> = self
            .shell_settings
            .default_env
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .chain(std::iter::once(format!(
                "{}={}",
                SESSION_ENV_VAR, session_id
            )))
            .collect();
        let env_refs: Vec<&str> = env_vars.iter().map(|s| s.as_str()).collect();

        // Create exec instance
        // Always wrap the command with sh -c to properly handle shell syntax
        let exec_options = CreateExecOptions {
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(tty),
            cmd: Some(vec!["sh", "-c", shell_cmd.as_str()]),
            env: Some(env_refs),
            ..Default::default()
        };

        let exec_creation = self
            .docker
            .create_exec(&container_id, exec_options)
            .await
            .map_err(|e| ShellServiceError::DockerOperationFailed {
                operation: "create exec".to_string(),
                message: e.to_string(),
            })?;

        let exec_id = exec_creation.id;

        // Start exec instance
        let start_options = StartExecOptions {
            detach: false,
            tty,
            ..Default::default()
        };

        let exec_stream = self
            .docker
            .start_exec(&exec_id, Some(start_options))
            .await
            .map_err(|e| ShellServiceError::DockerOperationFailed {
                operation: "start exec".to_string(),
                message: e.to_string(),
            })?;

        let (tx, rx) = mpsc::channel::<ShellCommand>(100);
        // Bounded, so a client sending faster than the process reads stdin is
        // slowed down instead of buffering without limit.
        let (stdin_tx, stdin_rx) = mpsc::channel::<ShellCommand>(STDIN_QUEUE_CAPACITY);

        // Create session info
        let session = ShellSession {
            session_id,
            app_name: app_data.name.clone(),
            service_name: service_name.to_string(),
            container_id: container_id.clone(),
            exec_id: exec_id.clone(),
            sender: tx,
            stdin_sender: stdin_tx,
            client_id,
            tty,
            created_at: chrono::Utc::now(),
        };

        // Store session
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(session_id, session);
        }

        // Record session started
        crate::metrics::shell::record_session_started();

        // Audit log: Shell session created
        info!(
            event = "shell_session_created",
            session_id = %session_id,
            client_id = %client_id,
            app_name = %app_data.name,
            service_name = %service_name,
            container_id = %container_id,
            shell_command = %shell_cmd,
            tty = tty,
            "Shell session created for compliance audit"
        );

        let ctx = SessionContext {
            session_id,
            client_id,
            app_name: app_data.name.clone(),
            service_name: service_name.to_string(),
            started: tokio::time::Instant::now(),
            app_state: app_state.clone(),
        };

        // Send session created message to client
        if let Err(e) = ctx
            .send(WebSocketMessage::ShellSessionCreated(
                ShellSessionInfo::new(
                    session_id,
                    app_data.name.clone(),
                    service_name.to_string(),
                    container_id.clone(),
                    shell_cmd,
                ),
            ))
            .await
        {
            warn!(
                "Failed to send session created message to client {}: {}",
                client_id, e
            );
            // Continue anyway - session is already created
        }

        let runner = SessionRunner {
            ctx,
            docker: self.docker.clone(),
            exec_id,
            container_id,
            tty,
            ttl: self.shell_settings.session_ttl(),
        };
        let active_sessions = self.active_sessions.clone();

        crate::metrics::spawn_instrumented(async move {
            let stop_processes = {
                // Create guard to ensure session cleanup even on panic
                let _guard = SessionGuard::new(session_id, active_sessions);
                runner.run(exec_stream, rx, stdin_rx).await
            };
            // Done after the session was removed, so stopping the processes
            // does not count against the session limits.
            if stop_processes {
                stop_session_processes(&runner.docker, &runner.container_id, session_id).await;
            }
        });

        Ok(session_id)
    }

    /// Look up a session owned by `client_id`.
    ///
    /// Returns `SessionNotFound` both for unknown sessions and for sessions
    /// owned by another WebSocket client, so callers cannot probe whether a
    /// foreign session exists.
    fn owned_session(
        sessions: &HashMap<Uuid, ShellSession>,
        session_id: Uuid,
        client_id: Uuid,
    ) -> ShellServiceResult<&ShellSession> {
        sessions
            .get(&session_id)
            .filter(|session| session.client_id == client_id)
            .ok_or(ShellServiceError::SessionNotFound { session_id })
    }

    /// Send input to a shell session owned by `client_id`
    pub async fn send_input(
        &self,
        session_id: Uuid,
        client_id: Uuid,
        input: Vec<u8>,
    ) -> ShellServiceResult<()> {
        let sessions = self.active_sessions.read().await;
        Self::owned_session(&sessions, session_id, client_id)?
            .send_command(ShellCommand::Input(input))
            .await
    }

    /// Resize the TTY of a shell session owned by `client_id`.
    ///
    /// Sessions without a TTY have nothing to resize; the request is ignored.
    pub async fn resize_tty(
        &self,
        session_id: Uuid,
        client_id: Uuid,
        width: u16,
        height: u16,
    ) -> ShellServiceResult<()> {
        let sessions = self.active_sessions.read().await;
        let session = Self::owned_session(&sessions, session_id, client_id)?;
        if !session.tty {
            return Ok(());
        }
        session
            .send_command(ShellCommand::Resize { width, height })
            .await
    }

    /// Close the stdin of a shell session owned by `client_id`, so the
    /// process reads end-of-file. Output keeps flowing until it exits.
    pub async fn close_stdin(&self, session_id: Uuid, client_id: Uuid) -> ShellServiceResult<()> {
        let sessions = self.active_sessions.read().await;
        Self::owned_session(&sessions, session_id, client_id)?
            .send_command(ShellCommand::CloseStdin)
            .await
    }

    /// Terminate a shell session owned by `client_id`
    pub async fn terminate_session(
        &self,
        session_id: Uuid,
        client_id: Uuid,
    ) -> ShellServiceResult<()> {
        let sessions = self.active_sessions.read().await;
        Self::owned_session(&sessions, session_id, client_id)?
            .send_command(ShellCommand::Terminate)
            .await
    }

    /// Terminate all shell sessions owned by a WebSocket client.
    ///
    /// Called when the client disconnects. Returns the IDs of the sessions
    /// that were asked to terminate.
    pub async fn terminate_client_sessions(&self, client_id: Uuid) -> Vec<Uuid> {
        let sessions = self.active_sessions.read().await;
        let client_sessions: Vec<_> = sessions
            .values()
            .filter(|s| s.client_id == client_id)
            .cloned()
            .collect();
        drop(sessions);

        let mut terminated = Vec::new();
        for session in client_sessions {
            if session.sender.send(ShellCommand::Terminate).await.is_ok() {
                terminated.push(session.session_id);
            }
        }
        terminated
    }

    /// Register a session without a Docker exec, for tests.
    ///
    /// Returns the receiving end of the session's command channel so tests
    /// can observe which commands reach the session.
    #[cfg(test)]
    pub(crate) async fn insert_test_session(
        &self,
        app_name: &str,
        client_id: Uuid,
    ) -> (Uuid, mpsc::Receiver<ShellCommand>) {
        self.insert_test_session_with_tty(app_name, client_id, true)
            .await
    }

    /// Like [`Self::insert_test_session`], for a session without a TTY.
    #[cfg(test)]
    pub(crate) async fn insert_test_session_without_tty(
        &self,
        app_name: &str,
        client_id: Uuid,
    ) -> (Uuid, mpsc::Receiver<ShellCommand>) {
        self.insert_test_session_with_tty(app_name, client_id, false)
            .await
    }

    #[cfg(test)]
    async fn insert_test_session_with_tty(
        &self,
        app_name: &str,
        client_id: Uuid,
        tty: bool,
    ) -> (Uuid, mpsc::Receiver<ShellCommand>) {
        // One channel for control and stdin, so tests observe every command.
        let (sender, receiver) = mpsc::channel(16);
        let session_id = Uuid::new_v4();
        let session = ShellSession {
            session_id,
            app_name: app_name.to_string(),
            service_name: "web".to_string(),
            container_id: "test-container".to_string(),
            exec_id: "test-exec".to_string(),
            stdin_sender: sender.clone(),
            sender,
            client_id,
            tty,
            created_at: chrono::Utc::now(),
        };
        self.active_sessions
            .write()
            .await
            .insert(session_id, session);
        (session_id, receiver)
    }

    /// Register a session without a Docker exec whose stdin queue holds
    /// `stdin_capacity` commands, like a real session, for tests.
    ///
    /// Returns the receiving ends of the control and the stdin channel.
    #[cfg(test)]
    pub(crate) async fn insert_test_session_with_stdin_queue(
        &self,
        app_name: &str,
        client_id: Uuid,
        stdin_capacity: usize,
    ) -> (
        Uuid,
        mpsc::Receiver<ShellCommand>,
        mpsc::Receiver<ShellCommand>,
    ) {
        let (sender, control) = mpsc::channel(16);
        let (stdin_sender, stdin) = mpsc::channel(stdin_capacity);
        let session_id = Uuid::new_v4();
        let session = ShellSession {
            session_id,
            app_name: app_name.to_string(),
            service_name: "web".to_string(),
            container_id: "test-container".to_string(),
            exec_id: "test-exec".to_string(),
            sender,
            stdin_sender,
            client_id,
            tty: false,
            created_at: chrono::Utc::now(),
        };
        self.active_sessions
            .write()
            .await
            .insert(session_id, session);
        (session_id, control, stdin)
    }

    /// Get active sessions
    #[allow(dead_code)]
    pub async fn get_active_sessions(&self) -> Vec<ShellSession> {
        let sessions = self.active_sessions.read().await;
        sessions.values().cloned().collect()
    }

    /// Terminate all sessions for an app
    #[allow(dead_code)]
    pub async fn terminate_app_sessions(&self, app_name: &str) {
        let sessions = self.active_sessions.read().await;
        let app_sessions: Vec<_> = sessions
            .values()
            .filter(|s| s.app_name == app_name)
            .cloned()
            .collect();
        drop(sessions);

        for session in app_sessions {
            let _ = session.sender.send(ShellCommand::Terminate).await;
        }
    }
}
