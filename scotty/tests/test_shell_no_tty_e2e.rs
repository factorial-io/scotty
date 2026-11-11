//! End-to-end tests for shell sessions without a tty (`docker exec -i` style).
//!
//! They drive the real WebSocket message dispatcher against a real container,
//! covering the `container-shell` spec: stdin forwarding, closing stdin,
//! separate stdout/stderr, binary-safe data and exit codes.
//!
//! Marked #[ignore] because they need a Docker daemon and the `alpine:3.20`
//! image. Run locally with:
//! cargo test -p scotty --test test_shell_no_tty_e2e -- --ignored --nocapture

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use axum::extract::ws::Message;
use scotty::api::auth_core::CurrentUser;
use scotty::api::test_utils::create_test_app_state_with_policy_dir;
use scotty::api::websocket::handlers::handle_websocket_message;
use scotty::app_state::{SharedAppState, WebSocketClient};
use scotty_core::apps::app_data::{AppData, ContainerState, ContainerStatus};
use scotty_core::websocket::message::WebSocketMessage;
use scotty_types::{ShellDataType, ShellSessionData, ShellSessionRequest};
use tokio::sync::broadcast;
use uuid::Uuid;

const APP: &str = "shell-e2e";
const IMAGE: &str = "alpine:3.20";
const TIMEOUT: Duration = Duration::from_secs(30);

/// A throwaway container, removed on drop.
struct TestContainer(String);

impl TestContainer {
    fn start() -> Self {
        let out = Command::new("docker")
            .args(["run", "-d", "--rm", IMAGE, "sleep", "600"])
            .output()
            .expect("docker must be installed");
        assert!(
            out.status.success(),
            "docker run failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        Self(String::from_utf8(out.stdout).unwrap().trim().to_string())
    }
}

impl Drop for TestContainer {
    fn drop(&mut self) {
        let _ = Command::new("docker").args(["rm", "-f", &self.0]).output();
    }
}

/// What a session produced until `ShellSessionEnded`.
#[derive(Debug, Default)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit_code: Option<i32>,
    /// Number of data messages without an `encoding`.
    plain_text_messages: usize,
    errors: Vec<String>,
}

/// Sends input for one client; usable while the harness reads output.
#[derive(Clone)]
struct InputSender {
    state: SharedAppState,
    client_id: Uuid,
}

impl InputSender {
    async fn input(&self, session_id: Uuid, bytes: &[u8]) {
        let msg = WebSocketMessage::ShellSessionData(ShellSessionData::binary(
            session_id,
            ShellDataType::Input,
            bytes,
        ));
        handle_websocket_message(&self.state, self.client_id, &msg).await;
    }

    async fn close_stdin(&self, session_id: Uuid) {
        let msg = WebSocketMessage::CloseShellStdin { session_id };
        handle_websocket_message(&self.state, self.client_id, &msg).await;
    }
}

struct Harness {
    state: SharedAppState,
    client_id: Uuid,
    rx: broadcast::Receiver<Message>,
    _policy_dir: tempfile::TempDir,
    _container: TestContainer,
}

impl Harness {
    async fn new() -> Self {
        // Large enough to hold every output message of the biggest test.
        Self::with_client_buffer(65536).await
    }

    /// `client_buffer` is the per-client message buffer, as in the real
    /// WebSocket handler.
    async fn with_client_buffer(client_buffer: usize) -> Self {
        // Work on a copy of the fixture policy: assigning app scopes writes
        // the policy file.
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
        let state = create_test_app_state_with_policy_dir(
            settings,
            None,
            policy_dir.path().to_str().unwrap(),
        )
        .await;

        let container = TestContainer::start();
        let web = ContainerState {
            status: ContainerStatus::Running,
            id: Some(container.0.clone()),
            service: "web".to_string(),
            ..Default::default()
        };
        state
            .apps
            .add_app(AppData::new(
                APP,
                "/tmp/shell-e2e",
                "/tmp/shell-e2e/docker-compose.yml",
                vec![web],
                None,
            ))
            .await
            .unwrap();
        state
            .auth_service
            .set_app_scopes(APP, vec!["default".to_string()])
            .await
            .unwrap();

        let client_id = Uuid::new_v4();
        let (tx, rx) = broadcast::channel(client_buffer);
        state
            .messenger
            .add_client(client_id, WebSocketClient::new(tx))
            .await;
        state
            .messenger
            .authenticate_client(
                client_id,
                CurrentUser {
                    email: "identifier:admin".to_string(),
                    name: "admin".to_string(),
                    picture: None,
                    access_token: None,
                },
            )
            .await
            .unwrap();

        Self {
            state,
            client_id,
            rx,
            _policy_dir: policy_dir,
            _container: container,
        }
    }

    fn input_sender(&self) -> InputSender {
        InputSender {
            state: self.state.clone(),
            client_id: self.client_id,
        }
    }

    async fn send(&self, msg: WebSocketMessage) {
        handle_websocket_message(&self.state, self.client_id, &msg).await;
    }

    async fn next_message(&mut self) -> WebSocketMessage {
        loop {
            let msg = tokio::time::timeout(TIMEOUT, self.rx.recv())
                .await
                .expect("timed out waiting for a WebSocket message")
                .expect("message channel closed or lagged: output was dropped");
            if let Message::Text(text) = msg {
                if let Ok(parsed) = serde_json::from_str(&text) {
                    return parsed;
                }
            }
        }
    }

    async fn create(&mut self, command: Option<&str>, tty: bool) -> Uuid {
        self.send(WebSocketMessage::CreateShellSession(ShellSessionRequest {
            app_name: APP.to_string(),
            service_name: "web".to_string(),
            shell_command: command.map(str::to_string),
            tty,
        }))
        .await;
        loop {
            match self.next_message().await {
                WebSocketMessage::ShellSessionCreated(info) => return info.session_id,
                WebSocketMessage::Error(err) => panic!("session creation failed: {err}"),
                _ => continue,
            }
        }
    }

    async fn input(&self, session_id: Uuid, bytes: &[u8]) {
        self.send(WebSocketMessage::ShellSessionData(
            ShellSessionData::binary(session_id, ShellDataType::Input, bytes),
        ))
        .await;
    }

    async fn close_stdin(&self, session_id: Uuid) {
        self.send(WebSocketMessage::CloseShellStdin { session_id })
            .await;
    }

    /// Record one message into `outcome`; returns true once the session ended.
    fn record(outcome: &mut Outcome, session_id: Uuid, msg: WebSocketMessage) -> bool {
        match msg {
            WebSocketMessage::ShellSessionData(data) if data.session_id == session_id => {
                if data.encoding.is_none() {
                    outcome.plain_text_messages += 1;
                }
                let bytes = data.bytes().expect("output must decode");
                match data.data_type {
                    ShellDataType::Stderr => outcome.stderr.extend(bytes),
                    _ => outcome.stdout.extend(bytes),
                }
                false
            }
            WebSocketMessage::ShellSessionEnded(end) if end.session_id == session_id => {
                outcome.exit_code = end.exit_code;
                true
            }
            WebSocketMessage::ShellSessionError(err) => {
                outcome.errors.push(err.error);
                false
            }
            WebSocketMessage::Error(err) => {
                outcome.errors.push(err);
                false
            }
            _ => false,
        }
    }

    async fn run_to_end(&mut self, session_id: Uuid) -> Outcome {
        let mut outcome = Outcome::default();
        loop {
            let msg = self.next_message().await;
            if Self::record(&mut outcome, session_id, msg) {
                return outcome;
            }
        }
    }

    /// Read stdout until it ends with `marker`; returns everything read.
    async fn read_stdout_until(&mut self, session_id: Uuid, marker: &[u8]) -> Vec<u8> {
        let mut outcome = Outcome::default();
        while !outcome.stdout.ends_with(marker) {
            let msg = self.next_message().await;
            assert!(
                !Self::record(&mut outcome, session_id, msg),
                "session ended before {:?} arrived; got {:?}",
                String::from_utf8_lossy(marker),
                outcome
            );
        }
        outcome.stdout
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn stdin_without_tty_runs_commands_and_propagates_exit_code() {
    let mut h = Harness::new().await;
    let session_id = h.create(Some("/bin/sh"), false).await;

    h.input(session_id, b"echo hello\nexit 3\n").await;
    h.close_stdin(session_id).await;
    let outcome = h.run_to_end(session_id).await;

    assert_eq!(String::from_utf8_lossy(&outcome.stdout), "hello\n");
    assert!(outcome.stderr.is_empty(), "{outcome:?}");
    assert_eq!(outcome.exit_code, Some(3));
    assert_eq!(
        outcome.plain_text_messages, 0,
        "non-tty data must be base64"
    );
    assert!(outcome.errors.is_empty(), "{outcome:?}");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn stdout_and_stderr_stay_separate() {
    let mut h = Harness::new().await;
    let session_id = h.create(Some("echo out; echo err >&2"), false).await;
    h.close_stdin(session_id).await;

    let outcome = h.run_to_end(session_id).await;

    assert_eq!(outcome.stdout, b"out\n");
    assert_eq!(outcome.stderr, b"err\n");
    assert_eq!(outcome.exit_code, Some(0));
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn closing_stdin_ends_cat_and_data_is_byte_exact() {
    let mut h = Harness::new().await;
    let session_id = h.create(Some("cat"), false).await;

    // Every byte value, repeated to span several Docker frames.
    let payload: Vec<u8> = (0..=255u8).cycle().take(256 * 1024).collect();
    for chunk in payload.chunks(16 * 1024) {
        h.input(session_id, chunk).await;
    }
    h.close_stdin(session_id).await;
    let outcome = h.run_to_end(session_id).await;

    assert_eq!(outcome.stdout.len(), payload.len());
    assert!(outcome.stdout == payload, "output differs from input");
    assert_eq!(outcome.exit_code, Some(0));
}

/// Input and output flow at the same time. `cat` echoes while it reads, so
/// the server must keep reading output while it is still writing input;
/// otherwise both pipes fill up and the session deadlocks.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn large_bidirectional_transfer_does_not_deadlock() {
    let mut h = Harness::new().await;
    let session_id = h.create(Some("cat"), false).await;

    let payload: Vec<u8> = (0..=255u8).cycle().take(16 * 1024 * 1024).collect();
    let sender = h.input_sender();
    let send_all = async {
        for chunk in payload.chunks(32 * 1024) {
            sender.input(session_id, chunk).await;
        }
        sender.close_stdin(session_id).await;
    };
    // Like a real client, read output while sending input.
    let (_, outcome) = tokio::time::timeout(
        Duration::from_secs(60),
        futures_util::future::join(send_all, h.run_to_end(session_id)),
    )
    .await
    .expect("transfer stalled: the session stopped forwarding output");

    assert_eq!(outcome.stdout.len(), payload.len());
    assert!(outcome.stdout == payload, "output differs from input");
    assert_eq!(outcome.exit_code, Some(0));
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn binary_output_is_preserved() {
    let mut h = Harness::new().await;
    let session_id = h.create(Some(r"printf '\000\377\200\n'"), false).await;
    h.close_stdin(session_id).await;

    let outcome = h.run_to_end(session_id).await;

    assert_eq!(outcome.stdout, [0x00, 0xff, 0x80, b'\n']);
    assert_eq!(outcome.exit_code, Some(0));
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn long_running_shell_is_driven_command_by_command() {
    let mut h = Harness::new().await;
    let session_id = h.create(Some("/bin/sh"), false).await;

    // The phabalicious pattern: write one command, read until a marker.
    h.input(session_id, b"echo first; echo __DONE__\n").await;
    let out = h.read_stdout_until(session_id, b"__DONE__\n").await;
    assert_eq!(out, b"first\n__DONE__\n");

    h.input(session_id, b"x=42; echo \"x=$x\"; echo __DONE__\n")
        .await;
    let out = h.read_stdout_until(session_id, b"__DONE__\n").await;
    assert_eq!(out, b"x=42\n__DONE__\n");

    h.input(session_id, b"exit 0\n").await;
    let outcome = h.run_to_end(session_id).await;
    assert_eq!(outcome.exit_code, Some(0));
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn command_without_stdin_sees_eof() {
    let mut h = Harness::new().await;
    // Without stdin forwarding the client closes stdin right away; `cat`
    // must end instead of waiting forever.
    let session_id = h.create(Some("cat; echo end"), false).await;
    h.close_stdin(session_id).await;

    let outcome = h.run_to_end(session_id).await;

    assert_eq!(outcome.stdout, b"end\n");
    assert_eq!(outcome.exit_code, Some(0));
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn input_after_closing_stdin_is_ignored() {
    let mut h = Harness::new().await;
    let session_id = h.create(Some("cat"), false).await;
    h.input(session_id, b"before\n").await;
    h.close_stdin(session_id).await;
    h.input(session_id, b"after\n").await;

    let outcome = h.run_to_end(session_id).await;

    assert_eq!(outcome.stdout, b"before\n");
    assert_eq!(outcome.exit_code, Some(0));
    assert!(outcome.errors.is_empty(), "{outcome:?}");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn tty_session_is_unchanged() {
    let mut h = Harness::new().await;
    let session_id = h.create(Some("printf 'a\\n'; exit 5"), true).await;

    let outcome = h.run_to_end(session_id).await;

    // A tty translates LF to CRLF and the data stays plain text.
    assert_eq!(String::from_utf8_lossy(&outcome.stdout), "a\r\n");
    assert!(outcome.plain_text_messages > 0);
    assert_eq!(outcome.exit_code, Some(5));
}

/// The process writes a lot of output before it reads stdin, while the
/// client is already sending input. The server must keep forwarding output
/// while the process is not reading stdin; a single loop that blocks on the
/// stdin write deadlocks here.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn output_before_reading_stdin_does_not_deadlock() {
    const OUTPUT: usize = 32 * 1024 * 1024;
    let mut h = Harness::new().await;
    let session_id = h
        .create(
            Some(&format!(
                "head -c {OUTPUT} /dev/zero; cat >/dev/null; echo done"
            )),
            false,
        )
        .await;

    let payload = vec![7u8; 8 * 1024 * 1024];
    let sender = h.input_sender();
    let send_all = async {
        for chunk in payload.chunks(32 * 1024) {
            sender.input(session_id, chunk).await;
        }
        sender.close_stdin(session_id).await;
    };
    // Like a real client, read output while sending input.
    let (_, outcome) = tokio::time::timeout(
        Duration::from_secs(60),
        futures_util::future::join(send_all, h.run_to_end(session_id)),
    )
    .await
    .expect("transfer stalled: the session stopped forwarding output");

    assert_eq!(outcome.stdout.len(), OUTPUT + "done\n".len());
    assert!(outcome.stdout.ends_with(b"done\n"));
    assert_eq!(outcome.exit_code, Some(0));
}

/// A client that reads output slower than the process writes it must still
/// get every byte: the server has to wait for the client instead of letting
/// the per-client message buffer overflow and drop output.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn slow_client_gets_all_output() {
    // Several thousand output messages: far more than the client buffer.
    const OUTPUT: usize = 128 * 1024 * 1024;
    // The same per-client buffer as the real WebSocket handler.
    let mut h = Harness::with_client_buffer(1000).await;
    let session_id = h
        .create(Some(&format!("head -c {OUTPUT} /dev/zero")), false)
        .await;
    h.close_stdin(session_id).await;

    // Let the process run ahead of the client before reading anything.
    tokio::time::sleep(Duration::from_secs(2)).await;
    let mut outcome = Outcome::default();
    loop {
        let msg = h.next_message().await;
        if Harness::record(&mut outcome, session_id, msg) {
            break;
        }
        tokio::time::sleep(Duration::from_micros(200)).await;
    }

    assert_eq!(outcome.stdout.len(), OUTPUT);
    assert_eq!(outcome.exit_code, Some(0));
}

/// Processes in the container that belong to a shell session.
fn session_processes(container: &str, session_id: Uuid) -> String {
    let script = format!(
        "for d in /proc/[0-9]*; do tr '\\0' '\\n' < $d/environ 2>/dev/null \
         | grep -qx SCOTTY_SHELL_SESSION={session_id} && cat $d/comm; done; true"
    );
    let out = Command::new("docker")
        .args(["exec", container, "sh", "-c", &script])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Wait until the session's processes are gone; returns what is left.
async fn wait_for_no_session_processes(container: &str, session_id: Uuid) -> String {
    let mut left = String::new();
    for _ in 0..50 {
        left = session_processes(container, session_id);
        if left.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    left
}

/// Docker does not kill an exec's process when its connection closes; without
/// a tty there is no hang-up either. Terminating a session must stop it.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn terminating_a_session_stops_its_processes() {
    let mut h = Harness::new().await;
    let session_id = h.create(Some("sleep 300 & sleep 300"), false).await;
    let container = h._container.0.clone();
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(
        !session_processes(&container, session_id).is_empty(),
        "the session's processes should be running"
    );

    h.send(WebSocketMessage::TerminateShellSession { session_id })
        .await;

    assert_eq!(
        wait_for_no_session_processes(&container, session_id).await,
        ""
    );
}

/// A client that disconnects (e.g. Ctrl-C on `app:shell -c 'tail -f ...'`)
/// must not leave the command running in the container.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn client_going_away_stops_the_processes() {
    let mut h = Harness::new().await;
    let session_id = h.create(Some("yes"), false).await;
    let container = h._container.0.clone();
    h.next_message().await;
    assert!(
        !session_processes(&container, session_id).is_empty(),
        "the session's processes should be running"
    );

    h.state.messenger.remove_client(h.client_id).await;

    assert_eq!(
        wait_for_no_session_processes(&container, session_id).await,
        ""
    );
}
