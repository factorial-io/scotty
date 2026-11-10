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

/// Helper to emit audit log for shell session termination
fn audit_log_session_ended(
    session_id: Uuid,
    client_id: Uuid,
    app_name: &str,
    service_name: &str,
    duration_secs: f64,
    reason: &str,
    exit_code: Option<i32>,
) {
    info!(
        event = "shell_session_ended",
        session_id = %session_id,
        client_id = %client_id,
        app_name = %app_name,
        service_name = %service_name,
        duration_seconds = duration_secs,
        reason = %reason,
        exit_code = ?exit_code,
        "Shell session ended for compliance audit"
    );
}

/// How often and how long to poll Docker for the exit code once the output
/// stream of an exec has closed. Docker may still report the exec as running
/// for a moment after its streams are closed.
const EXIT_CODE_POLL_INTERVAL: Duration = Duration::from_millis(50);
const EXIT_CODE_POLL_ATTEMPTS: u32 = 60;

/// Read the exit code of a finished exec, waiting briefly for Docker to
/// record it.
async fn wait_for_exit_code(docker: &Docker, exec_id: &str) -> Option<i32> {
    for _ in 0..EXIT_CODE_POLL_ATTEMPTS {
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
    warn!(
        "Exec {} still reported as running after its output closed",
        exec_id
    );
    None
}

/// Number of stdin chunks queued per session before the client is slowed down
const STDIN_QUEUE_CAPACITY: usize = 32;

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
            ShellCommand::CloseStdin | ShellCommand::Resize { .. } | ShellCommand::Terminate => {}
        }
    }
    Ok(())
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

    /// Create a new shell session
    pub async fn create_session(
        &self,
        app_state: &SharedAppState,
        app_data: &AppData,
        service_name: &str,
        shell_command: Option<String>,
        tty: bool,
        client_id: Uuid, // WebSocket client ID
    ) -> ShellServiceResult<Uuid> {
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
            })?;

        // Generate session ID
        let session_id = Uuid::new_v4();

        // Determine shell command to use
        let shell_cmd = shell_command.unwrap_or_else(|| self.shell_settings.default_shell.clone());

        // Prepare environment variables
        let env_vars: Vec<String> = self
            .shell_settings
            .default_env
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
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
            env: if env_refs.is_empty() {
                None
            } else {
                Some(env_refs)
            },
            ..Default::default()
        };

        let exec_creation = self
            .docker
            .create_exec(container_id, exec_options)
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

        let (tx, mut rx) = mpsc::channel::<ShellCommand>(100);
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
            sessions.insert(session_id, session.clone());
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

        // Clone values we need to move into the spawned task
        let app_name_clone = app_data.name.clone();
        let service_name_clone = service_name.to_string();
        let container_id_clone = container_id.clone();
        let shell_cmd_clone = shell_cmd.clone();

        // Send session created message to client
        if let Err(e) = app_state
            .messenger
            .send_to_client(
                client_id,
                WebSocketMessage::ShellSessionCreated(ShellSessionInfo::new(
                    session_id,
                    app_name_clone.clone(),
                    service_name_clone.clone(),
                    container_id_clone.clone(),
                    shell_cmd_clone.clone(),
                )),
            )
            .await
        {
            warn!(
                "Failed to send session created message to client {}: {}",
                client_id, e
            );
            // Continue anyway - session is already created
        }

        // Start the shell session handler
        let docker = self.docker.clone();
        let app_state = app_state.clone();
        let active_sessions = self.active_sessions.clone();
        let session_ttl = self.shell_settings.session_ttl();

        crate::metrics::spawn_instrumented(async move {
            // Create guard to ensure session cleanup even on panic
            let _guard = SessionGuard::new(session_id, active_sessions.clone());

            info!(
                "Starting shell session {} for container {}",
                session_id, container_id_clone
            );

            match exec_stream {
                StartExecResults::Attached { input, mut output } => {
                    let session_start = tokio::time::Instant::now();
                    let mut status_check_interval =
                        tokio::time::interval(tokio::time::Duration::from_millis(500));
                    status_check_interval
                        .set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                    let mut stdin_writer = tokio::spawn(write_stdin(session_id, input, stdin_rx));
                    let mut stdin_writer_done = false;

                    loop {
                        tokio::select! {
                            // Check for TTL timeout
                            _ = tokio::time::sleep_until(session_start + session_ttl) => {
                                info!("Shell session {} expired after TTL", session_id);
                                let duration_secs = session_start.elapsed().as_secs_f64();
                                crate::metrics::shell::record_session_timeout(duration_secs);

                                // Audit log session end
                                audit_log_session_ended(
                                    session_id,
                                    client_id,
                                    &app_name_clone,
                                    &service_name_clone,
                                    duration_secs,
                                    "Session timeout",
                                    None,
                                );

                                if let Err(e) = app_state.messenger.send_to_client(
                                    client_id,
                                    WebSocketMessage::ShellSessionEnded(ShellSessionEnd {
                                        session_id,
                                        exit_code: None,
                                        reason: "Session timeout".to_string(),
                                    }),
                                ).await {
                                    warn!("Failed to send timeout message to client {}: {}", client_id, e);
                                }
                                break;
                            }

                            // Periodically check if exec is still running. Only for
                            // TTY mode, where a background process can keep the pty
                            // (and therefore the output stream) open after the shell
                            // exited. Without a TTY the output stream closes when the
                            // process exits, after all output was delivered.
                            _ = status_check_interval.tick(), if tty => {
                                match docker.inspect_exec(&exec_id).await {
                                    Ok(exec_info) => {
                                        if let Some(running) = exec_info.running {
                                            if !running {
                                                info!("Shell session {} detected exec completion via status check", session_id);
                                                let duration_secs = session_start.elapsed().as_secs_f64();
                                                crate::metrics::shell::record_session_ended(duration_secs);
                                                let exit_code = exec_info.exit_code.map(|c| c as i32);

                                                // Audit log session end
                                                audit_log_session_ended(
                                                    session_id,
                                                    client_id,
                                                    &app_name_clone,
                                                    &service_name_clone,
                                                    duration_secs,
                                                    "Shell exited",
                                                    exit_code,
                                                );

                                                if let Err(e) = app_state.messenger.send_to_client(
                                                    client_id,
                                                    WebSocketMessage::ShellSessionEnded(ShellSessionEnd {
                                                        session_id,
                                                        exit_code,
                                                        reason: "Shell exited".to_string(),
                                                    }),
                                                ).await {
                                                    warn!("Failed to send session end message to client {}: {}", client_id, e);
                                                }
                                                break;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to inspect exec {} for session {}: {}", exec_id, session_id, e);
                                    }
                                }
                            }

                            // The stdin writer stopped: report a write failure.
                            // A clean stop (queue closed) needs no action.
                            result = &mut stdin_writer, if !stdin_writer_done => {
                                stdin_writer_done = true;
                                let error = match result {
                                    Ok(Ok(())) => continue,
                                    Ok(Err(e)) => e.to_string(),
                                    Err(e) => format!("stdin writer failed: {}", e),
                                };
                                error!("Failed to write to shell {}: {}", session_id, error);
                                let duration_secs = session_start.elapsed().as_secs_f64();
                                crate::metrics::shell::record_session_error(duration_secs);

                                // Audit log session end due to error
                                audit_log_session_ended(
                                    session_id,
                                    client_id,
                                    &app_name_clone,
                                    &service_name_clone,
                                    duration_secs,
                                    &format!("Error writing input: {}", error),
                                    None,
                                );

                                if let Err(send_err) = app_state.messenger.send_to_client(
                                    client_id,
                                    WebSocketMessage::ShellSessionError(ShellSessionError {
                                        session_id,
                                        error: format!("Failed to write input: {}", error),
                                    }),
                                ).await {
                                    warn!("Failed to send error message to client {}: {}", client_id, send_err);
                                }
                                break;
                            }

                            // Handle control commands
                            Some(cmd) = rx.recv() => {
                                match cmd {
                                    // Routed to the stdin writer by send_command
                                    ShellCommand::Input(_) | ShellCommand::CloseStdin => {}
                                    ShellCommand::Resize { width, height } => {
                                        // Resize the TTY
                                        if let Err(e) = docker.resize_exec(&exec_id, bollard::exec::ResizeExecOptions {
                                            height,
                                            width,
                                        }).await {
                                            warn!("Failed to resize TTY for session {}: {}", session_id, e);
                                        }
                                    }
                                    ShellCommand::Terminate => {
                                        info!("Terminating shell session {} by request", session_id);
                                        let duration_secs = session_start.elapsed().as_secs_f64();
                                        crate::metrics::shell::record_session_ended(duration_secs);

                                        // Audit log session end
                                        audit_log_session_ended(
                                            session_id,
                                            client_id,
                                            &app_name_clone,
                                            &service_name_clone,
                                            duration_secs,
                                            "Terminated by user",
                                            None,
                                        );

                                        if let Err(e) = app_state.messenger.send_to_client(
                                            client_id,
                                            WebSocketMessage::ShellSessionEnded(ShellSessionEnd {
                                                session_id,
                                                exit_code: None,
                                                reason: "Terminated by user".to_string(),
                                            }),
                                        ).await {
                                            warn!("Failed to send termination message to client {}: {}", client_id, e);
                                        }
                                        break;
                                    }
                                }
                            }

                            // Read output from shell stream
                            result = output.next() => {
                                match result {
                                    Some(Ok(log_output)) => {
                                        use bollard::container::LogOutput;
                                        let (data_type, message) = match log_output {
                                            LogOutput::StdOut { message } => (ShellDataType::Output, message),
                                            LogOutput::StdErr { message } => (ShellDataType::Stderr, message),
                                            // TTY mode sends output as Console
                                            LogOutput::Console { message } => (ShellDataType::Output, message),
                                            LogOutput::StdIn { .. } => continue, // Skip stdin
                                        };
                                        // With a TTY, keep plain text for existing clients.
                                        // Without one, send the exact bytes.
                                        let data = if tty {
                                            ShellSessionData::text(
                                                session_id,
                                                data_type,
                                                String::from_utf8_lossy(&message),
                                            )
                                        } else {
                                            ShellSessionData::binary(session_id, data_type, &message)
                                        };

                                        // If we can't send output, client is gone - terminate session
                                        if let Err(e) = app_state.messenger.send_to_client(
                                            client_id,
                                            WebSocketMessage::ShellSessionData(data),
                                        ).await {
                                            warn!("Failed to send output to client {}: {} - terminating session", client_id, e);
                                            break;
                                        }
                                    }
                                    Some(Err(e)) => {
                                        error!("Failed to read from shell stream {}: {}", session_id, e);
                                        let duration_secs = session_start.elapsed().as_secs_f64();
                                        crate::metrics::shell::record_session_error(duration_secs);
                                        if let Err(send_err) = app_state.messenger.send_to_client(
                                            client_id,
                                            WebSocketMessage::ShellSessionError(ShellSessionError {
                                                session_id,
                                                error: format!("Failed to read output: {}", e),
                                            }),
                                        ).await {
                                            warn!("Failed to send error message to client {}: {}", client_id, send_err);
                                        }
                                        break;
                                    }
                                    // Stream closed: the process exited and all of
                                    // its output has been forwarded.
                                    None => {
                                        info!("Shell session {} ended (stream closed)", session_id);
                                        // Blocks this loop for at most a few seconds;
                                        // with the output closed there is nothing left
                                        // to forward, and control commands can wait.
                                        let exit_code = wait_for_exit_code(&docker, &exec_id).await;
                                        let duration_secs = session_start.elapsed().as_secs_f64();
                                        crate::metrics::shell::record_session_ended(duration_secs);

                                        audit_log_session_ended(
                                            session_id,
                                            client_id,
                                            &app_name_clone,
                                            &service_name_clone,
                                            duration_secs,
                                            "Shell exited",
                                            exit_code,
                                        );

                                        if let Err(e) = app_state.messenger.send_to_client(
                                            client_id,
                                            WebSocketMessage::ShellSessionEnded(ShellSessionEnd {
                                                session_id,
                                                exit_code,
                                                reason: "Shell exited".to_string(),
                                            }),
                                        ).await {
                                            warn!("Failed to send session end message to client {}: {}", client_id, e);
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    // The session is over; stop writing to the exec.
                    stdin_writer.abort();
                }
                StartExecResults::Detached => {
                    drop(stdin_rx);
                    error!(
                        "Shell session {} started in detached mode (unexpected)",
                        session_id
                    );
                    crate::metrics::shell::record_session_error(0.0);
                    if let Err(e) = app_state
                        .messenger
                        .send_to_client(
                            client_id,
                            WebSocketMessage::ShellSessionError(ShellSessionError {
                                session_id,
                                error: "Shell started in detached mode".to_string(),
                            }),
                        )
                        .await
                    {
                        warn!(
                            "Failed to send detached error message to client {}: {}",
                            client_id, e
                        );
                    }
                }
            }

            // Session cleanup is handled automatically by SessionGuard on drop
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
