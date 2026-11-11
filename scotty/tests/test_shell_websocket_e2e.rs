//! End-to-end WebSocket integration tests for shell sessions
//!
//! These tests verify the full WebSocket flow including:
//! - Authentication and authorization
//! - Shell session creation
//! - Bidirectional message exchange
//! - Session lifecycle management
//!
//! Note: These tests are marked with #[ignore] to prevent execution in CI
//! because they require:
//! - Docker daemon running
//! - Test containers available
//! - Network access
//!
//! Run locally with: cargo test --test test_shell_websocket_e2e -- --ignored --nocapture

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use scotty_core::apps::app_data::{AppData, ContainerState, ContainerStatus};
use scotty_core::websocket::message::WebSocketMessage;
use scotty_types::{ShellDataType, ShellSessionRequest};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

const APP: &str = "shell-ws-e2e";
const IMAGE: &str = "alpine:3.20";
/// `identifier:admin` in tests/test_bearer_auth.yaml.
const ADMIN_TOKEN: &str = "test-bearer-token-123";
const TIMEOUT: Duration = Duration::from_secs(30);

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
type WsWrite = futures_util::stream::SplitSink<WsStream, Message>;
/// Incoming messages; panics instead of waiting forever when none arrives.
type WsRead = std::pin::Pin<
    Box<dyn futures_util::Stream<Item = tokio_tungstenite::tungstenite::Result<Message>> + Send>,
>;

/// A throwaway container, removed on drop.
struct TestContainer(String);

impl Drop for TestContainer {
    fn drop(&mut self) {
        let _ = Command::new("docker").args(["rm", "-f", &self.0]).output();
    }
}

/// Keeps the server, container and policy copy alive for one test.
struct Fixture {
    _server: axum_test::TestServer,
    _container: TestContainer,
    _policy_dir: tempfile::TempDir,
}

/// Start a real HTTP server with app `APP` backed by a running container,
/// open a WebSocket on `/ws` and authenticate it as admin.
async fn connect() -> (Fixture, WsWrite, WsRead) {
    let out = Command::new("docker")
        .args(["run", "-d", "--rm", IMAGE, "sleep", "600"])
        .output()
        .expect("docker must be installed");
    assert!(
        out.status.success(),
        "docker run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let container = TestContainer(String::from_utf8(out.stdout).unwrap().trim().to_string());

    // Work on a copy of the fixture policy: assigning app scopes writes the
    // policy file.
    let policy_dir = tempfile::tempdir().unwrap();
    for file in ["model.conf", "policy.yaml"] {
        std::fs::copy(
            Path::new("tests/fixtures/casbin").join(file),
            policy_dir.path().join(file),
        )
        .unwrap();
    }
    let settings: scotty::settings::config::Settings = config::Config::builder()
        .add_source(config::File::with_name("tests/test_bearer_auth"))
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();
    let app_state = scotty::api::test_utils::create_test_app_state_with_policy_dir(
        settings,
        None,
        policy_dir.path().to_str().unwrap(),
    )
    .await;
    let web = ContainerState {
        status: ContainerStatus::Running,
        id: Some(container.0.clone()),
        service: "web".to_string(),
        ..Default::default()
    };
    app_state
        .apps
        .add_app(AppData::new(
            APP,
            "/tmp/shell-ws-e2e",
            "/tmp/shell-ws-e2e/docker-compose.yml",
            vec![web],
            None,
        ))
        .await
        .unwrap();
    app_state
        .auth_service
        .set_app_scopes(APP, vec!["default".to_string()])
        .await
        .unwrap();

    // A real socket: the default mock transport has no address to connect to.
    let server = axum_test::TestServer::builder()
        .http_transport()
        .build(scotty::api::router::ApiRoutes::create(app_state));
    let mut url = server.server_address().expect("Server should have address");
    url.set_scheme("ws").unwrap();
    url.set_path("/ws");
    let (ws_stream, _) = connect_async(url.as_str())
        .await
        .expect("Failed to connect to WebSocket");
    let (mut write, mut read) = ws_stream.split();

    let auth = WebSocketMessage::Authenticate {
        token: ADMIN_TOKEN.to_string(),
    };
    write
        .send(Message::Text(serde_json::to_string(&auth).unwrap().into()))
        .await
        .expect("Failed to send authentication");
    loop {
        let msg = tokio::time::timeout(TIMEOUT, read.next())
            .await
            .expect("timed out waiting for authentication")
            .expect("WebSocket closed before authentication")
            .expect("WebSocket error");
        if let Message::Text(text) = msg {
            match serde_json::from_str(&text) {
                Ok(WebSocketMessage::AuthenticationSuccess) => break,
                Ok(WebSocketMessage::AuthenticationFailed { reason }) => {
                    panic!("authentication failed: {reason}")
                }
                _ => continue,
            }
        }
    }

    let read: WsRead = Box::pin(futures_util::stream::unfold(read, |mut read| async {
        tokio::time::timeout(TIMEOUT, read.next())
            .await
            .expect("timed out waiting for a WebSocket message")
            .map(|msg| (msg, read))
    }));
    let fixture = Fixture {
        _server: server,
        _container: container,
        _policy_dir: policy_dir,
    };
    (fixture, write, read)
}

/// Test basic shell session creation and termination flow
///
/// This test verifies:
/// - WebSocket connection establishment
/// - Authentication (if required)
/// - Shell session creation request/response
/// - Session ended notification
///
/// Marked #[ignore] - requires Docker
#[tokio::test]
#[ignore]
async fn test_shell_session_creation_and_termination() {
    let (_fixture, mut write, mut read) = connect().await;

    // Create shell session request
    let session_request = ShellSessionRequest {
        app_name: APP.to_string(),
        service_name: "web".to_string(),
        shell_command: Some("/bin/sh".to_string()),
        tty: true,
    };

    let create_message = WebSocketMessage::CreateShellSession(session_request);
    let json = serde_json::to_string(&create_message).expect("Failed to serialize message");

    // Send session creation request
    write
        .send(Message::Text(json.into()))
        .await
        .expect("Failed to send create session message");

    // Wait for session created response
    let mut session_id: Option<Uuid> = None;
    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                match ws_msg {
                    WebSocketMessage::ShellSessionCreated(info) => {
                        session_id = Some(info.session_id);
                        println!("Shell session created: {}", info.session_id);
                        break;
                    }
                    WebSocketMessage::Error(err) => {
                        panic!("Failed to create shell session: {}", err);
                    }
                    _ => continue,
                }
            }
        }
    }

    assert!(
        session_id.is_some(),
        "Should have received ShellSessionCreated message"
    );

    // Send terminate command (via Ctrl+D or exit)
    let input_message = WebSocketMessage::ShellSessionData(scotty_types::ShellSessionData {
        session_id: session_id.unwrap(),
        data_type: ShellDataType::Input,
        data: "exit\n".to_string(),
        encoding: None,
    });

    let json = serde_json::to_string(&input_message).expect("Failed to serialize input");
    write
        .send(Message::Text(json.into()))
        .await
        .expect("Failed to send exit command");

    // Wait for session ended
    let mut session_ended = false;
    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                match ws_msg {
                    WebSocketMessage::ShellSessionEnded(end) => {
                        println!("Session ended: {} - {}", end.session_id, end.reason);
                        session_ended = true;
                        break;
                    }
                    WebSocketMessage::ShellSessionData(_) => {
                        // Ignore output data
                        continue;
                    }
                    _ => continue,
                }
            }
        }
    }

    assert!(session_ended, "Should have received ShellSessionEnded");
}

/// Test command execution with exit code verification
///
/// This test verifies:
/// - Command execution in non-interactive mode
/// - Output data streaming
/// - Exit code propagation in ShellSessionEnded message
///
/// Marked #[ignore] - requires Docker
#[tokio::test]
#[ignore]
async fn test_shell_command_execution_with_exit_code() {
    let (_fixture, mut write, mut read) = connect().await;

    // Create shell session with command that will exit with code 42
    let session_request = ShellSessionRequest {
        app_name: APP.to_string(),
        service_name: "web".to_string(),
        shell_command: Some("exit 42".to_string()),
        tty: true,
    };

    let create_message = WebSocketMessage::CreateShellSession(session_request);
    let json = serde_json::to_string(&create_message).expect("Failed to serialize message");

    write
        .send(Message::Text(json.into()))
        .await
        .expect("Failed to send create session message");

    // Collect messages
    let mut session_created = false;
    let mut exit_code: Option<i32> = None;

    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                match ws_msg {
                    WebSocketMessage::ShellSessionCreated(info) => {
                        println!("Shell session created: {}", info.session_id);
                        session_created = true;
                    }
                    WebSocketMessage::ShellSessionEnded(end) => {
                        println!(
                            "Session ended with exit code: {:?} - {}",
                            end.exit_code, end.reason
                        );
                        exit_code = end.exit_code;
                        break;
                    }
                    WebSocketMessage::ShellSessionData(data) => {
                        println!("Received output: {}", data.data);
                    }
                    WebSocketMessage::Error(err) => {
                        println!("Error: {}", err);
                    }
                    _ => continue,
                }
            }
        }
    }

    assert!(session_created, "Should have created session");
    assert_eq!(
        exit_code,
        Some(42),
        "Should have received exit code 42 from command"
    );
}

/// Test input/output bidirectional flow
///
/// This test verifies:
/// - Sending input to shell
/// - Receiving output from shell
/// - Multiple input/output exchanges
///
/// Marked #[ignore] - requires Docker
#[tokio::test]
#[ignore]
async fn test_shell_bidirectional_io() {
    let (_fixture, mut write, mut read) = connect().await;

    // Create interactive shell session
    let session_request = ShellSessionRequest {
        app_name: APP.to_string(),
        service_name: "web".to_string(),
        shell_command: None, // Interactive mode
        tty: true,
    };

    let create_message = WebSocketMessage::CreateShellSession(session_request);
    let json = serde_json::to_string(&create_message).expect("Failed to serialize message");

    write
        .send(Message::Text(json.into()))
        .await
        .expect("Failed to send create session message");

    // Wait for session created
    let mut session_id: Option<Uuid> = None;
    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                match ws_msg {
                    WebSocketMessage::ShellSessionCreated(info) => {
                        session_id = Some(info.session_id);
                        break;
                    }
                    _ => continue,
                }
            }
        }
    }

    let session_id = session_id.expect("Should have session ID");

    // Send echo command
    let echo_input = WebSocketMessage::ShellSessionData(scotty_types::ShellSessionData {
        session_id,
        data_type: ShellDataType::Input,
        data: "echo hello-$((40+2))\n".to_string(),
        encoding: None,
    });

    let json = serde_json::to_string(&echo_input).expect("Failed to serialize input");
    write
        .send(Message::Text(json.into()))
        .await
        .expect("Failed to send echo command");

    // Collect output until the command's result arrives. The tty echoes the
    // input too, but only the shell's arithmetic produces "hello-42".
    let mut received_output = String::new();
    while !received_output.contains("hello-42") {
        let msg = read
            .next()
            .await
            .expect("WebSocket closed before the output arrived");
        if let Ok(Message::Text(text)) = msg {
            if let Ok(WebSocketMessage::ShellSessionData(data)) =
                serde_json::from_str::<WebSocketMessage>(&text)
            {
                if matches!(data.data_type, ShellDataType::Output) {
                    received_output.push_str(&data.data);
                }
            }
        }
    }

    println!("Total output received: {}", received_output);
}

/// Test TTY resize message
///
/// This test verifies:
/// - Sending TTY resize commands
/// - Server accepting resize without error
///
/// Marked #[ignore] - requires Docker
#[tokio::test]
#[ignore]
async fn test_shell_tty_resize() {
    let (_fixture, mut write, mut read) = connect().await;

    // Create shell session
    let session_request = ShellSessionRequest {
        app_name: APP.to_string(),
        service_name: "web".to_string(),
        shell_command: None,
        tty: true,
    };

    let create_message = WebSocketMessage::CreateShellSession(session_request);
    let json = serde_json::to_string(&create_message).expect("Failed to serialize message");

    write
        .send(Message::Text(json.into()))
        .await
        .expect("Failed to send create session message");

    // Wait for session created
    let mut session_id: Option<Uuid> = None;
    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(WebSocketMessage::ShellSessionCreated(info)) =
                serde_json::from_str::<WebSocketMessage>(&text)
            {
                session_id = Some(info.session_id);
                break;
            }
        }
    }

    let session_id = session_id.expect("Should have session ID");

    // Send resize message
    let resize_message = WebSocketMessage::ResizeShellTty {
        session_id,
        width: 120,
        height: 40,
    };

    let json = serde_json::to_string(&resize_message).expect("Failed to serialize resize");
    write
        .send(Message::Text(json.into()))
        .await
        .expect("Failed to send resize message");

    // Resize is fire-and-forget; errors would come back as messages.
    let quiet = tokio::time::sleep(Duration::from_millis(500));
    tokio::pin!(quiet);
    loop {
        tokio::select! {
            _ = &mut quiet => break,
            msg = read.next() => {
                let Some(Ok(Message::Text(text))) = msg else { continue };
                match serde_json::from_str::<WebSocketMessage>(&text) {
                    Ok(WebSocketMessage::Error(err)) => panic!("resize failed: {err}"),
                    Ok(WebSocketMessage::ShellSessionError(err)) => {
                        panic!("resize failed: {}", err.error)
                    }
                    _ => continue,
                }
            }
        }
    }
}
