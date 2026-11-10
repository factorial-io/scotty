use anyhow::Context;
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, size as terminal_size},
};
use futures_util::StreamExt;
use std::io::{stdout, IsTerminal, Read, Write};
use tokio::select;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error};

use crate::{cli::ShellCommand, context::AppContext};
use scotty_core::websocket::message::WebSocketMessage;
use scotty_types::{ShellDataType, ShellSessionData};
use uuid::Uuid;

/// How a shell session handles stdin and the remote tty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ShellMode {
    /// Forward local stdin to the remote process
    forward_stdin: bool,
    /// Allocate a tty on the remote side
    tty: bool,
}

/// Work out stdin forwarding and tty allocation, like `ssh` and `docker exec`:
/// without `--command` stdin is always forwarded and a tty is allocated only
/// when stdin is a terminal; with `--command` both are opt-in (`-i`, `-t`).
fn resolve_mode(cmd: &ShellCommand, stdin_is_terminal: bool) -> anyhow::Result<ShellMode> {
    let forward_stdin = cmd.interactive || cmd.command.is_none();
    let tty = if cmd.tty {
        true
    } else if cmd.no_tty {
        false
    } else {
        cmd.command.is_none() && stdin_is_terminal
    };
    if tty && forward_stdin && !stdin_is_terminal {
        anyhow::bail!(
            "An interactive tty needs a terminal on stdin. \
             Omit -t, or use -T to forward stdin without a tty."
        );
    }
    Ok(ShellMode { forward_stdin, tty })
}

/// Open an interactive shell for an app service
pub async fn shell_app(context: &AppContext, cmd: &ShellCommand) -> anyhow::Result<()> {
    let mode = resolve_mode(cmd, std::io::stdin().is_terminal())?;
    // Without a tty the output is data: keep status messages out of the way
    // unless someone is watching stderr.
    let show_status = mode.tty || std::io::stderr().is_terminal();

    // Validate app and service using shared utility
    let _app_data = super::validate_app_and_service_with_status(
        context,
        &cmd.app_name,
        &cmd.service_name,
        "app:shell",
        show_status,
    )
    .await?;

    // Create shell session and open interactive terminal
    open_shell(context, cmd, mode, show_status).await
}

/// Create shell session and open interactive terminal
async fn open_shell(
    context: &AppContext,
    cmd: &ShellCommand,
    mode: ShellMode,
    show_status: bool,
) -> anyhow::Result<()> {
    use crate::websocket::AuthenticatedWebSocket;
    use scotty_types::ShellSessionRequest;

    let ui = context.ui();

    // Connect to WebSocket first
    ui.new_status_line("Connecting to WebSocket...");
    let mut ws = AuthenticatedWebSocket::connect(context.server())
        .await
        .context("Failed to connect to WebSocket")?;

    if show_status {
        ui.success("🔐 WebSocket authenticated");
    }

    // Build shell command:
    // - If command is provided, just pass it (server will wrap with sh -c)
    // - If no command, pass the custom shell for interactive mode
    let shell_command = if cmd.command.is_some() {
        // For command mode, just pass the command - server will wrap it
        cmd.command.clone()
    } else {
        // For interactive mode, pass custom shell or use default
        cmd.shell.clone()
    };

    // Create shell session via WebSocket
    ui.new_status_line("Creating shell session...");

    let request = ShellSessionRequest {
        app_name: cmd.app_name.clone(),
        service_name: cmd.service_name.clone(),
        shell_command,
        tty: mode.tty,
    };

    // Send CreateShellSession message
    let message = WebSocketMessage::CreateShellSession(request);
    ws.send(message)
        .await
        .context("Failed to send shell session creation request")?;

    // Wait for ShellSessionCreated response
    use futures_util::StreamExt;
    let session_id = loop {
        match ws.receiver.next().await {
            Some(Ok(Message::Text(text))) => {
                if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                    match ws_message {
                        WebSocketMessage::ShellSessionCreated(info) => {
                            if show_status {
                                ui.success(format!("Shell session created: {}", info.session_id));
                            }
                            break info.session_id;
                        }
                        WebSocketMessage::Error(err) => {
                            anyhow::bail!("Failed to create shell session: {}", err);
                        }
                        _ => {
                            // Ignore other messages while waiting for session creation
                            continue;
                        }
                    }
                }
            }
            Some(Ok(Message::Close(_))) => {
                anyhow::bail!("WebSocket closed while waiting for session creation");
            }
            Some(Err(e)) => {
                anyhow::bail!("WebSocket error: {}", e);
            }
            None => {
                anyhow::bail!("WebSocket stream ended");
            }
            _ => continue,
        }
    };

    // Without a tty, stream stdin and output as raw bytes
    if !mode.tty {
        ui.clear();
        return run_stream_mode(ws, session_id, mode.forward_stdin).await;
    }

    // A tty without stdin: just wait for output and exit
    if !mode.forward_stdin {
        return run_command_mode(ws, session_id).await;
    }

    // Send initial terminal size before entering interactive mode
    if let Ok((cols, rows)) = terminal_size() {
        debug!("Sending initial terminal size: {}x{}", cols, rows);
        let resize_message = WebSocketMessage::ResizeShellTty {
            session_id,
            width: cols,
            height: rows,
        };
        if let Err(e) = ws.send(resize_message).await {
            error!("Failed to send initial terminal size: {}", e);
            // Continue anyway - terminal might work without proper size
        }
    }

    // Otherwise, run interactive mode
    run_interactive_mode(ws, session_id, ui).await
}

/// Size of the chunks read from stdin in stream mode
const STDIN_CHUNK_SIZE: usize = 32 * 1024;

/// Read stdin on a blocking thread and hand chunks to the async side.
/// The channel closes after end-of-file; a read error is passed on so it is
/// not mistaken for the end of the input.
fn spawn_stdin_reader() -> tokio::sync::mpsc::Receiver<std::io::Result<Vec<u8>>> {
    let (tx, rx) = tokio::sync::mpsc::channel(8);
    std::thread::spawn(move || {
        let mut stdin = std::io::stdin().lock();
        let mut buf = vec![0u8; STDIN_CHUNK_SIZE];
        loop {
            let chunk = match stdin.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => Ok(buf[..n].to_vec()),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => Err(e),
            };
            let failed = chunk.is_err();
            if tx.blocking_send(chunk).is_err() || failed {
                break;
            }
        }
    });
    rx
}

/// Forward stdin to the session and close the remote stdin at end-of-file
/// (right away when stdin is not forwarded).
///
/// Runs in its own task so the client keeps reading output while a send
/// waits: the server slows down input when the process does not read stdin,
/// and output it cannot deliver slows down the process. A client that stops
/// reading while it sends would block itself. Returns the sender so the
/// connection stays open after the input is done.
async fn forward_stdin(
    mut sender: crate::websocket::WsSender,
    session_id: Uuid,
    stdin: Option<tokio::sync::mpsc::Receiver<std::io::Result<Vec<u8>>>>,
) -> Result<crate::websocket::WsSender, String> {
    use futures_util::SinkExt;

    async fn send(
        sender: &mut crate::websocket::WsSender,
        message: WebSocketMessage,
    ) -> Result<(), String> {
        let json = serde_json::to_string(&message).map_err(|e| e.to_string())?;
        sender
            .send(Message::Text(json.into()))
            .await
            .map_err(|e| e.to_string())
    }

    if let Some(mut stdin) = stdin {
        while let Some(chunk) = stdin.recv().await {
            let bytes = chunk.map_err(|e| format!("Failed to read stdin: {}", e))?;
            let data = ShellSessionData::binary(session_id, ShellDataType::Input, &bytes);
            send(&mut sender, WebSocketMessage::ShellSessionData(data))
                .await
                .map_err(|e| format!("Failed to send input: {}", e))?;
        }
        debug!("stdin reached end-of-file, closing remote stdin");
    }
    send(
        &mut sender,
        WebSocketMessage::CloseShellStdin { session_id },
    )
    .await
    .map_err(|e| format!("Failed to close remote stdin: {}", e))?;
    Ok(sender)
}

/// Run a session without a tty: forward stdin as raw bytes, close the
/// remote stdin on end-of-file, and pass stdout and stderr through unchanged.
/// Exits the process with the remote exit code.
async fn run_stream_mode(
    ws: crate::websocket::AuthenticatedWebSocket,
    session_id: Uuid,
    forward: bool,
) -> anyhow::Result<()> {
    let crate::websocket::AuthenticatedWebSocket {
        sender,
        mut receiver,
    } = ws;
    let stdin = forward.then(spawn_stdin_reader);
    let mut input_task = tokio::spawn(forward_stdin(sender, session_id, stdin));
    let mut input_done = false;
    // Keeps the connection's sending half alive once the input is done.
    let mut _sender = None;

    let mut out = std::io::stdout();
    let mut err = std::io::stderr();

    let exit_code = loop {
        select! {
            result = &mut input_task, if !input_done => {
                input_done = true;
                match result {
                    Ok(Ok(sender)) => _sender = Some(sender),
                    Ok(Err(e)) => {
                        eprintln!("{}", e);
                        break 1;
                    }
                    Err(e) => {
                        eprintln!("Forwarding stdin failed: {}", e);
                        break 1;
                    }
                }
            }

            message = receiver.next() => {
                match message {
                    Some(Ok(Message::Text(text))) => {
                        let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) else {
                            continue;
                        };
                        match ws_message {
                            WebSocketMessage::ShellSessionData(data) if data.session_id == session_id => {
                                let bytes = match data.bytes() {
                                    Ok(bytes) => bytes,
                                    Err(e) => {
                                        eprintln!("Server sent invalid shell output: {}", e);
                                        break 1;
                                    }
                                };
                                let written = match data.data_type {
                                    ShellDataType::Stderr => err.write_all(&bytes).and_then(|_| err.flush()),
                                    _ => out.write_all(&bytes).and_then(|_| out.flush()),
                                };
                                match written {
                                    Ok(()) => {}
                                    // The reader went away (e.g. `| head`): stop quietly,
                                    // like a process killed by SIGPIPE.
                                    Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => break 141,
                                    Err(e) => {
                                        eprintln!("Failed to write shell output: {}", e);
                                        break 1;
                                    }
                                }
                            }
                            WebSocketMessage::ShellSessionEnded(end) if end.session_id == session_id => {
                                break match end.exit_code {
                                    Some(code) => code,
                                    None => {
                                        eprintln!("Shell session ended: {}", end.reason);
                                        1
                                    }
                                };
                            }
                            WebSocketMessage::ShellSessionError(error) => {
                                eprintln!("Shell error: {}", error.error);
                                break 1;
                            }
                            WebSocketMessage::Error(msg) => {
                                eprintln!("Error: {}", msg);
                                break 1;
                            }
                            _ => {}
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        eprintln!("Connection to the server closed before the shell session ended");
                        break 1;
                    }
                    Some(Err(e)) => {
                        eprintln!("WebSocket error: {}", e);
                        break 1;
                    }
                    _ => {}
                }
            }
        }
    };

    // Exiting closes the connection; the server ends the session if it is
    // still running.
    std::process::exit(exit_code);
}

/// Run a single command and exit (non-interactive mode)
async fn run_command_mode(
    mut ws: crate::websocket::AuthenticatedWebSocket,
    _session_id: Uuid,
) -> anyhow::Result<()> {
    // Track exit code to propagate to CLI
    let mut exit_code: i32 = 0;

    // Just listen for output and print it
    while let Some(message) = ws.receiver.next().await {
        match message {
            Ok(Message::Text(text)) => {
                if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                    match ws_message {
                        WebSocketMessage::ShellSessionData(data) => {
                            // Print shell output directly
                            print!("{}", data.data);
                            let _ = stdout().flush();
                        }
                        WebSocketMessage::ShellSessionEnded(end) => {
                            // Capture exit code from shell session
                            exit_code = end.exit_code.unwrap_or(0);
                            break;
                        }
                        WebSocketMessage::ShellSessionError(error) => {
                            eprintln!("Shell error: {}", error.error);
                            exit_code = 1;
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Err(e) => {
                error!("WebSocket error: {}", e);
                exit_code = 1;
                break;
            }
            _ => {}
        }
    }

    let _ = ws.close().await;

    // Exit with the shell's exit code
    std::process::exit(exit_code);
}

/// Run interactive shell mode with raw terminal
async fn run_interactive_mode(
    mut ws: crate::websocket::AuthenticatedWebSocket,
    session_id: Uuid,
    ui: &crate::utils::ui::Ui,
) -> anyhow::Result<()> {
    // Clear the UI status line before entering raw mode
    ui.clear();

    // Enable raw mode for terminal
    enable_raw_mode().context("Failed to enable raw mode")?;

    // Ensure we disable raw mode on exit
    let result = run_interactive_loop(&mut ws, session_id).await;

    disable_raw_mode().context("Failed to disable raw mode")?;

    // Close WebSocket
    let _ = ws.close().await;

    result
}

/// Main interactive loop handling bidirectional I/O
async fn run_interactive_loop(
    ws: &mut crate::websocket::AuthenticatedWebSocket,
    session_id: Uuid,
) -> anyhow::Result<()> {
    let mut event_stream = EventStream::new();
    let mut should_exit = false;

    while !should_exit {
        select! {
            // Handle keyboard events
            maybe_event = event_stream.next() => {
                match maybe_event {
                    Some(Ok(event)) => {
                        if let Some(exit) = handle_terminal_event(event, ws, session_id).await? {
                            should_exit = exit;
                        }
                    }
                    Some(Err(e)) => {
                        error!("Error reading terminal event: {}", e);
                        break;
                    }
                    None => break,
                }
            }

            // Handle WebSocket messages
            maybe_message = ws.receiver.next() => {
                match maybe_message {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                            if let Some(exit) = handle_websocket_message(ws_message).await? {
                                should_exit = exit;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        debug!("WebSocket closed by server");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

/// Handle terminal input events
async fn handle_terminal_event(
    event: Event,
    ws: &mut crate::websocket::AuthenticatedWebSocket,
    session_id: Uuid,
) -> anyhow::Result<Option<bool>> {
    match event {
        Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }) => {
            // Ctrl+C - send interrupt to shell
            debug!("Ctrl+C pressed");
            // Send Ctrl+C as raw byte
            let input_data = ShellSessionData::text(session_id, ShellDataType::Input, "\x03");
            let message = WebSocketMessage::ShellSessionData(input_data);
            ws.send(message).await?;
            Ok(None)
        }
        Event::Key(KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }) => {
            // Ctrl+D - exit interactive mode
            debug!("Ctrl+D pressed, exiting");
            Ok(Some(true))
        }
        Event::Key(KeyEvent {
            code, modifiers, ..
        }) => {
            // Convert key event to string and send to shell
            if let Some(input) = key_to_string(code, modifiers) {
                let input_data = ShellSessionData::text(session_id, ShellDataType::Input, input);
                let message = WebSocketMessage::ShellSessionData(input_data);
                ws.send(message).await?;
            }
            Ok(None)
        }
        Event::Resize(width, height) => {
            // Send terminal resize event
            debug!("Terminal resized to {}x{}", width, height);
            let resize_message = WebSocketMessage::ResizeShellTty {
                session_id,
                width,
                height,
            };
            ws.send(resize_message).await?;
            Ok(None)
        }
        _ => Ok(None),
    }
}

/// Handle WebSocket messages from server
async fn handle_websocket_message(message: WebSocketMessage) -> anyhow::Result<Option<bool>> {
    match message {
        WebSocketMessage::ShellSessionData(data) => {
            // Print shell output directly to stdout
            print!("{}", data.data);
            let _ = stdout().flush();
            Ok(None)
        }
        WebSocketMessage::ShellSessionEnded(end) => {
            println!("\r\nShell session ended: {}", end.reason);
            Ok(Some(true))
        }
        WebSocketMessage::ShellSessionError(error) => {
            eprintln!("\r\nShell error: {}", error.error);
            Ok(Some(true))
        }
        WebSocketMessage::Error(msg) => {
            eprintln!("\r\nWebSocket error: {}", msg);
            Ok(Some(true))
        }
        _ => Ok(None),
    }
}

/// Convert crossterm key codes to string input
fn key_to_string(code: KeyCode, modifiers: KeyModifiers) -> Option<String> {
    match code {
        KeyCode::Char(c) => {
            if modifiers.contains(KeyModifiers::CONTROL) {
                // Handle other Ctrl combinations if needed
                None
            } else {
                Some(c.to_string())
            }
        }
        KeyCode::Enter => Some("\r".to_string()),
        KeyCode::Backspace => Some("\x7f".to_string()),
        KeyCode::Tab => Some("\t".to_string()),
        KeyCode::Esc => Some("\x1b".to_string()),
        KeyCode::Up => Some("\x1b[A".to_string()),
        KeyCode::Down => Some("\x1b[B".to_string()),
        KeyCode::Right => Some("\x1b[C".to_string()),
        KeyCode::Left => Some("\x1b[D".to_string()),
        KeyCode::Home => Some("\x1b[H".to_string()),
        KeyCode::End => Some("\x1b[F".to_string()),
        KeyCode::PageUp => Some("\x1b[5~".to_string()),
        KeyCode::PageDown => Some("\x1b[6~".to_string()),
        KeyCode::Delete => Some("\x1b[3~".to_string()),
        KeyCode::Insert => Some("\x1b[2~".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell_cmd(args: &[&str]) -> ShellCommand {
        use clap::Parser;
        let mut argv = vec!["app:shell", "my-app", "web"];
        argv.extend_from_slice(args);
        ShellCommand::try_parse_from(argv).unwrap()
    }

    fn mode(forward_stdin: bool, tty: bool) -> ShellMode {
        ShellMode { forward_stdin, tty }
    }

    #[test]
    fn test_mode_interactive_shell_from_terminal() {
        assert_eq!(
            resolve_mode(&shell_cmd(&[]), true).unwrap(),
            mode(true, true)
        );
    }

    #[test]
    fn test_mode_piped_stdin_without_command_has_no_tty() {
        assert_eq!(
            resolve_mode(&shell_cmd(&[]), false).unwrap(),
            mode(true, false)
        );
    }

    #[test]
    fn test_mode_command_has_neither_stdin_nor_tty_by_default() {
        for terminal in [true, false] {
            assert_eq!(
                resolve_mode(&shell_cmd(&["-c", "ls"]), terminal).unwrap(),
                mode(false, false)
            );
        }
    }

    #[test]
    fn test_mode_command_with_interactive_forwards_stdin() {
        assert_eq!(
            resolve_mode(&shell_cmd(&["-i", "-c", "cat"]), false).unwrap(),
            mode(true, false)
        );
    }

    #[test]
    fn test_mode_command_with_tty_needs_no_terminal() {
        assert_eq!(
            resolve_mode(&shell_cmd(&["-t", "-c", "top"]), false).unwrap(),
            mode(false, true)
        );
    }

    #[test]
    fn test_mode_no_tty_from_terminal() {
        assert_eq!(
            resolve_mode(&shell_cmd(&["-T"]), true).unwrap(),
            mode(true, false)
        );
    }

    #[test]
    fn test_mode_interactive_tty_without_terminal_is_an_error() {
        let err = resolve_mode(&shell_cmd(&["-t"]), false).unwrap_err();
        assert!(err.to_string().contains("-T"), "{err}");
        let err = resolve_mode(&shell_cmd(&["-it", "-c", "sh"]), false).unwrap_err();
        assert!(err.to_string().contains("terminal"), "{err}");
    }

    #[test]
    fn test_mode_tty_and_no_tty_conflict() {
        use clap::Parser;
        assert!(ShellCommand::try_parse_from(["app:shell", "a", "b", "-t", "-T"]).is_err());
    }

    /// Test key_to_string with printable characters
    #[test]
    fn test_key_to_string_printable_chars() {
        // Regular letters
        assert_eq!(
            key_to_string(KeyCode::Char('a'), KeyModifiers::empty()),
            Some("a".to_string())
        );
        assert_eq!(
            key_to_string(KeyCode::Char('Z'), KeyModifiers::empty()),
            Some("Z".to_string())
        );

        // Numbers
        assert_eq!(
            key_to_string(KeyCode::Char('5'), KeyModifiers::empty()),
            Some("5".to_string())
        );

        // Special characters
        assert_eq!(
            key_to_string(KeyCode::Char('!'), KeyModifiers::empty()),
            Some("!".to_string())
        );
        assert_eq!(
            key_to_string(KeyCode::Char('@'), KeyModifiers::empty()),
            Some("@".to_string())
        );
    }

    /// Test key_to_string with special keys (Enter, Backspace, Tab, Esc)
    #[test]
    fn test_key_to_string_special_keys() {
        // Enter should produce \r (carriage return)
        assert_eq!(
            key_to_string(KeyCode::Enter, KeyModifiers::empty()),
            Some("\r".to_string())
        );

        // Backspace should produce DEL character (0x7f)
        assert_eq!(
            key_to_string(KeyCode::Backspace, KeyModifiers::empty()),
            Some("\x7f".to_string())
        );

        // Tab should produce \t
        assert_eq!(
            key_to_string(KeyCode::Tab, KeyModifiers::empty()),
            Some("\t".to_string())
        );

        // Escape should produce ESC character (0x1b)
        assert_eq!(
            key_to_string(KeyCode::Esc, KeyModifiers::empty()),
            Some("\x1b".to_string())
        );
    }

    /// Test key_to_string with arrow keys (ANSI escape sequences)
    #[test]
    fn test_key_to_string_arrow_keys() {
        // Arrow keys produce ANSI escape sequences
        assert_eq!(
            key_to_string(KeyCode::Up, KeyModifiers::empty()),
            Some("\x1b[A".to_string())
        );
        assert_eq!(
            key_to_string(KeyCode::Down, KeyModifiers::empty()),
            Some("\x1b[B".to_string())
        );
        assert_eq!(
            key_to_string(KeyCode::Right, KeyModifiers::empty()),
            Some("\x1b[C".to_string())
        );
        assert_eq!(
            key_to_string(KeyCode::Left, KeyModifiers::empty()),
            Some("\x1b[D".to_string())
        );
    }

    /// Test key_to_string with navigation keys (Home, End, PageUp, PageDown, Delete, Insert)
    #[test]
    fn test_key_to_string_navigation_keys() {
        assert_eq!(
            key_to_string(KeyCode::Home, KeyModifiers::empty()),
            Some("\x1b[H".to_string())
        );
        assert_eq!(
            key_to_string(KeyCode::End, KeyModifiers::empty()),
            Some("\x1b[F".to_string())
        );
        assert_eq!(
            key_to_string(KeyCode::PageUp, KeyModifiers::empty()),
            Some("\x1b[5~".to_string())
        );
        assert_eq!(
            key_to_string(KeyCode::PageDown, KeyModifiers::empty()),
            Some("\x1b[6~".to_string())
        );
        assert_eq!(
            key_to_string(KeyCode::Delete, KeyModifiers::empty()),
            Some("\x1b[3~".to_string())
        );
        assert_eq!(
            key_to_string(KeyCode::Insert, KeyModifiers::empty()),
            Some("\x1b[2~".to_string())
        );
    }

    /// Test key_to_string with control modifiers (should return None)
    #[test]
    fn test_key_to_string_control_modifiers() {
        // Ctrl+C and Ctrl+D are handled separately, should return None here
        assert_eq!(
            key_to_string(KeyCode::Char('c'), KeyModifiers::CONTROL),
            None
        );
        assert_eq!(
            key_to_string(KeyCode::Char('d'), KeyModifiers::CONTROL),
            None
        );

        // Other Ctrl combinations should also return None (not implemented)
        assert_eq!(
            key_to_string(KeyCode::Char('a'), KeyModifiers::CONTROL),
            None
        );
        assert_eq!(
            key_to_string(KeyCode::Char('z'), KeyModifiers::CONTROL),
            None
        );
    }

    /// Test key_to_string with unsupported keys
    #[test]
    fn test_key_to_string_unsupported_keys() {
        // Function keys should return None
        assert_eq!(key_to_string(KeyCode::F(1), KeyModifiers::empty()), None);
        assert_eq!(key_to_string(KeyCode::F(12), KeyModifiers::empty()), None);

        // Media keys should return None
        assert_eq!(
            key_to_string(
                KeyCode::Media(crossterm::event::MediaKeyCode::Play),
                KeyModifiers::empty()
            ),
            None
        );

        // Null key should return None
        assert_eq!(key_to_string(KeyCode::Null, KeyModifiers::empty()), None);
    }

    /// Test that Ctrl+C produces the correct interrupt signal
    #[test]
    fn test_ctrl_c_interrupt_signal() {
        // Verify \x03 is the correct ETX (End of Text) / Ctrl+C signal
        let interrupt = "\x03";
        assert_eq!(interrupt.as_bytes(), &[0x03]);
        assert_eq!(interrupt.len(), 1);
    }

    /// Test ANSI escape sequence formatting
    #[test]
    fn test_ansi_escape_sequences() {
        // Verify escape sequences start with ESC (0x1b)
        let up_arrow = "\x1b[A";
        assert!(up_arrow.starts_with('\x1b'));
        assert_eq!(up_arrow.as_bytes(), &[0x1b, b'[', b'A']);

        // Verify Delete has tilde terminator
        let delete = "\x1b[3~";
        assert!(delete.ends_with('~'));
        assert_eq!(delete.as_bytes(), &[0x1b, b'[', b'3', b'~']);
    }
}
