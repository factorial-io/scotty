# Bollard Technical Spike - Findings and Recommendations

## Executive Summary

Successfully validated bollard's capabilities for implementing unified output system with real-time log streaming and interactive shell access. Bollard provides all necessary features to replace the current docker-compose command approach with a more robust, native Rust solution.

## Key Findings

### ✅ Container Logs API Capabilities

**Perfect for Unified Output System:**
- **Stream Separation**: Clear distinction between `LogOutput::StdOut` and `LogOutput::StdErr`
- **Timestamps**: Native timestamp support with `timestamps: true` option
- **Real-time Streaming**: `follow: true` enables live log following
- **Historical Limits**: `tail` parameter controls log history
- **Time-Synchronized**: Streams maintain chronological order when timestamps enabled

**API Structure:**
```rust
use bollard::query_parameters::LogsOptions;

let options = LogsOptions {
    stdout: true,
    stderr: true,
    follow: true,           // Real-time streaming
    tail: "100".to_string(), // Last N lines
    timestamps: true,       // Include timestamps
    ..Default::default()
};

let mut stream = docker.logs(&container_id, Some(options));
while let Some(log_result) = stream.next().await {
    match log_result? {
        LogOutput::StdOut { message } => {
            // Handle stdout with timestamp
        }
        LogOutput::StdErr { message } => {
            // Handle stderr with timestamp
        }
        _ => {}
    }
}
```

### ✅ Container Exec API Capabilities

**Fully Supports Interactive Shell Requirements:**
- **Session Management**: Each exec gets unique session ID
- **TTY Support**: `tty: true` enables proper terminal behavior
- **Bidirectional Communication**: `attach_stdin/stdout/stderr: true`
- **Command Flexibility**: Can specify any shell (`/bin/bash`, `/bin/sh`, etc.)

**API Structure:**
```rust
use bollard::exec::{CreateExecOptions, StartExecOptions, StartExecResults};

// Create interactive shell session
let exec_options = CreateExecOptions {
    cmd: Some(vec!["/bin/bash"]),
    attach_stdin: Some(true),
    attach_stdout: Some(true),
    attach_stderr: Some(true),
    tty: Some(true),
    ..Default::default()
};

let exec = docker.create_exec(&container_id, exec_options).await?;

// Start session for bidirectional communication
match docker.start_exec(&exec.id, Some(StartExecOptions {
    detach: false,
    tty: true,
    output_capacity: None,
})).await? {
    StartExecResults::Attached { mut output, mut input } => {
        // Handle bidirectional stream
    }
}
```

### ✅ Service Discovery and Container Mapping

**Existing Pattern Analysis:**
Scotty already has robust service discovery through:

1. **Docker Compose Integration**: Uses `docker-compose ps -q -a` to find containers
2. **Label-Based Service Mapping**: Extracts service name from `com.docker.compose.service` label
3. **Container Inspection**: Uses `bollard::Docker::inspect_container()` for detailed info

**Service → Container ID Mapping Process:**
```rust
// 1. Get containers for docker-compose app
let output = run_docker_compose_now(file, &["ps", "-q", "-a"], None, false)?;
let containers: Vec<String> = output.lines().map(String::from).collect();

// 2. Inspect each container to get service mapping
for container_id in containers {
    let insights = docker.inspect_container(&container_id, None).await?;
    let labels = insights.config.unwrap().labels.unwrap();
    let service_name = labels.get("com.docker.compose.service").unwrap();

    // service_name -> container_id mapping established
}
```

## Bollard vs Docker-Compose Commands Comparison

| Feature | Bollard (Recommended) | Docker-Compose Commands |
|---------|----------------------|------------------------|
| **Logs Streaming** | ✅ Native async streams | ⚠️ Subprocess management |
| **Stdout/Stderr Separation** | ✅ Built-in `LogOutput` enum | ⚠️ Manual stream handling |
| **Timestamps** | ✅ Native support | ⚠️ Docker-compose dependent |
| **Interactive Shell** | ✅ Full TTY support | ⚠️ Complex pseudo-TTY setup |
| **Error Handling** | ✅ Rust Result types | ⚠️ Exit codes + stderr parsing |
| **Performance** | ✅ Direct Docker API | ⚠️ Process spawn overhead |
| **Resource Usage** | ✅ Single connection pool | ⚠️ Multiple processes |
| **Integration** | ✅ Already used in codebase | ⚠️ Mixed approach |

## Service Discovery Strategy

**For app:logs and app:shell commands:**

```rust
pub async fn find_container_for_service(
    app_state: &SharedAppState,
    app_name: &str,
    service_name: &str,
) -> anyhow::Result<Option<String>> {
    // 1. Get app data (already contains service->container mapping)
    let apps = app_state.apps.read().await;
    let app = apps.apps.iter()
        .find(|a| a.name == app_name)
        .ok_or_else(|| anyhow!("App not found: {}", app_name))?;

    // 2. Find service and get container ID
    let service = app.services.iter()
        .find(|s| s.service == service_name)
        .ok_or_else(|| anyhow!("Service not found: {}", service_name))?;

    Ok(service.id.clone())
}
```

**Key Benefits:**
- ✅ Reuses existing service discovery logic
- ✅ No additional docker-compose calls needed
- ✅ Consistent with current app management
- ✅ Handles multiple instances per service

## Implementation Recommendations

### Phase 1: Core Unified Output System

1. **Replace TaskDetails stdout/stderr** with new unified model:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputLine {
    pub timestamp: DateTime<Utc>,
    pub stream: OutputStreamType, // Stdout | Stderr
    pub content: String,
    pub sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutput {
    pub task_id: Uuid,
    pub lines: VecDeque<OutputLine>,
    pub max_lines: usize,
}
```

2. **Update TaskManager** to use unified output collection:
```rust
async fn collect_unified_output(
    task_details: Arc<RwLock<TaskDetails>>,
    stdout: impl AsyncRead + Unpin,
    stderr: impl AsyncRead + Unpin,
) {
    // Use tokio::select! to coordinate both streams
    // Maintain chronological order with timestamps
    // Apply line limits and cleanup
}
```

### Phase 2: Bollard Log Streaming Service

```rust
pub struct LogStreamingService {
    docker: Docker,
    active_streams: Arc<RwLock<HashMap<Uuid, LogStreamSession>>>,
}

pub struct LogStreamSession {
    session_id: Uuid,
    app_name: String,
    service_name: String,
    container_id: String,
    // WebSocket sender for real-time updates
}

impl LogStreamingService {
    pub async fn start_log_stream(
        &self,
        app_name: &str,
        service_name: &str,
        options: LogStreamOptions,
    ) -> Result<Uuid> {
        // 1. Find container ID for service
        // 2. Create bollard LogsOptions
        // 3. Start streaming with WebSocket integration
        // 4. Return session ID for client tracking
    }
}
```

### Phase 3: Bollard Shell Service

```rust
pub struct ShellService {
    docker: Docker,
    active_sessions: Arc<RwLock<HashMap<Uuid, ShellSession>>>,
}

pub struct ShellSession {
    session_id: Uuid,
    exec_id: String,
    app_name: String,
    service_name: String,
    container_id: String,
    created_at: DateTime<Utc>,
    last_activity: DateTime<Utc>,
    ttl: Duration,
}

impl ShellService {
    pub async fn create_shell_session(
        &self,
        app_name: &str,
        service_name: &str,
        shell_options: ShellOptions,
    ) -> Result<ShellSession> {
        // 1. Find container ID for service
        // 2. Create bollard exec session
        // 3. Setup TTY and bidirectional communication
        // 4. Return session for WebSocket integration
    }
}
```

## WebSocket Integration Strategy

**Message Types for Real-time Communication:**
```rust
#[derive(Serialize, Deserialize, Debug)]
pub enum WebSocketMessage {
    // Existing messages preserved...

    // Log streaming
    LogStreamStart { app_name: String, service_name: String, session_id: Uuid },
    LogLine { session_id: Uuid, line: OutputLine },
    LogStreamEnd { session_id: Uuid, reason: String },

    // Shell sessions
    ShellSessionCreated { session_id: Uuid, app_name: String, service_name: String },
    ShellData { session_id: Uuid, data: Vec<u8> },
    ShellInput { session_id: Uuid, data: Vec<u8> },
    ShellSessionEnded { session_id: Uuid, exit_code: Option<i32> },
}
```

## CLI Integration with WebSocket

**For real-time streaming in CLI:**
- Use `tokio-tungstenite` for WebSocket client
- Handle terminal I/O with `crossterm`
- Coordinate WebSocket messages with local terminal state

## Performance Considerations

**Memory Management:**
- Implement circular buffers for log lines
- Configurable limits per stream/session
- Automatic cleanup of expired sessions

**Connection Pooling:**
- Reuse existing `SharedAppState.docker` client
- No additional connection overhead

**Concurrency:**
- Multiple log streams can run simultaneously
- Shell sessions are independent per container
- WebSocket broadcasts scale to multiple clients

## Risk Assessment

**Low Risk ✅:**
- Bollard already used extensively in codebase
- Service discovery logic already proven
- WebSocket infrastructure already exists
- Breaking changes are acceptable per requirements

**Medium Risk ⚠️:**
- CLI WebSocket integration complexity
- Terminal escape sequence handling in shells
- Session cleanup and TTL management

**Mitigation Strategies:**
- Start with log streaming (simpler)
- Extensive testing with different container types
- Fallback to docker-compose commands if needed
- Comprehensive session management testing

## Next Steps

1. **Immediate**: Begin implementing unified output data model
2. **Week 1**: Bollard log streaming service with WebSocket integration
3. **Week 2**: CLI app:logs command with real-time support
4. **Week 3**: Bollard shell service and session management
5. **Week 4**: CLI app:shell command with terminal integration
6. **Week 5**: Frontend unified log viewer and shell preparation

## Conclusion

Bollard provides all necessary capabilities for implementing the unified output system. The existing service discovery mechanism is robust and ready for integration. The technical spike confirms feasibility with low implementation risk and significant benefits over the docker-compose command approach.

**Recommendation: Proceed with bollard-based implementation as outlined in the PRD.**