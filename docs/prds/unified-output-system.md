# PRD: Unified Output System for Logs and Interactive Shell Access

## Overview

This document outlines the design for a unified output system that addresses the current time-synchronicity issues with stdout/stderr handling and introduces new capabilities for real-time log streaming and interactive shell access to Docker services.

## Current Problems

### Time Synchronicity Issues
- TaskDetails collects stdout and stderr in separate async tasks without coordination
- Temporal order of interleaved output is lost
- Simple string accumulation without timestamps
- Frontend displays two separate output sections, losing the actual execution flow

### Limited Functionality
- No real-time log streaming capability
- No interactive shell access to services
- Output accumulation without size limits can cause memory issues
- WebSocket infrastructure exists but isn't leveraged for real-time output

## Goals

### Primary Goals
1. **Unified Time-Synchronized Output**: Preserve the chronological order of stdout/stderr streams
2. **Real-time Log Streaming**: Enable `app:logs <service>` command with live following capability
3. **Interactive Shell Access**: Enable `app:shell <service>` command for direct container access
4. **Memory Management**: Implement configurable output limits with proper cleanup
5. **Permission Integration**: Leverage existing Shell and Logs permissions

### Secondary Goals
1. **Frontend Integration**: Unified log viewer in the web UI
2. **CLI WebSocket Support**: Enable real-time streaming in CLI tools
3. **Future Shell UI**: Plan for xterm.js integration (implementation later)

## Technical Approach

### Docker Integration Strategy

**Primary: Bollard**
- Use bollard's Container Logs API for streaming logs
- Use bollard's Exec API for interactive shell sessions
- Leverage existing bollard dependency and async streaming capabilities
- Better integration with Rust ecosystem and existing Docker client

**Fallback: Docker Compose Commands**
- Use `docker-compose logs -f` for log streaming if bollard approach faces issues
- Use `docker-compose exec -it` for shell access

### WebSocket Architecture

**Current WebSocket Infrastructure**:
- Existing broadcast-based WebSocket system in `api/ws.rs`
- Client management with UUID-based connection tracking
- Message types: Ping, AppListUpdated, TaskListUpdated, TaskInfoUpdated
- Frontend integration in `lib/ws.ts`

**Extensions Needed**:
- New message types for logs and shell data
- Per-client session management for shell connections
- Binary data support for shell escape sequences

**CLI WebSocket Integration**:
- Use existing WebSocket library (tokio-tungstenite recommended)
- Real-time streaming will work for both web UI and CLI
- CLI handles terminal escape sequences using crossterm or similar

## Data Models

### Unified Output Model

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputLine {
    pub timestamp: DateTime<Utc>,
    pub stream: OutputStreamType,
    pub content: String,
    pub sequence: u64,  // For ordering guarantee
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputStreamType {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingTaskOutput {
    pub task_id: Uuid,
    pub app_name: String,
    pub service_name: Option<String>,
    pub lines: VecDeque<OutputLine>,
    pub max_lines: usize,
    pub total_lines: u64,  // For pagination
}
```

### Separated Task Management

```rust
// Existing TaskDetails updated for task state management only
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDetails {
    pub id: Uuid,
    pub command: String,
    pub state: State,
    pub start_time: DateTime<Utc>,
    pub finish_time: Option<DateTime<Utc>>,
    pub last_exit_code: Option<i32>,
    pub app_name: Option<String>,
    // stdout/stderr fields removed - breaking change
}

// New separate structure for command output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutput {
    pub task_id: Uuid,
    pub output_lines: VecDeque<OutputLine>,
    pub max_lines: usize,
}
```

### Shell Session Management

```rust
#[derive(Debug, Clone)]
pub struct ShellSession {
    pub id: Uuid,
    pub app_name: String,
    pub service_name: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub ttl: Duration,
}

#[derive(Debug)]
pub struct ShellManager {
    sessions: Arc<RwLock<HashMap<Uuid, ShellSession>>>,
    // bollard exec handles per session
    exec_handles: Arc<RwLock<HashMap<Uuid, bollard::exec::CreateExecResults>>>,
}
```

## API Design

### New REST Endpoints

```rust
// Logs API
GET /apps/{app_name}/logs/{service}?follow=true&lines=100&since=timestamp
GET /apps/{app_name}/logs/{service}/download  // Download full logs

// Shell API
POST /apps/{app_name}/shell/{service}  // Create shell session
DELETE /apps/{app_name}/shell/{session_id}  // Terminate session
GET /apps/{app_name}/shell/sessions  // List active sessions

// Task Output API - NOT IMPLEMENTED
// Decision: Use WebSocket-only approach for unified experience
// Task output is streamed via WebSocket messages (TaskOutputData)
```

### WebSocket Protocol Extensions

```rust
#[derive(Serialize, Deserialize, Debug)]
pub enum WebSocketMessage {
    // Existing messages...
    Ping,
    Pong,
    AppListUpdated,
    TaskListUpdated,
    TaskInfoUpdated(TaskDetails),
    Error(String),

    // New log streaming messages
    LogLineReceived {
        app_name: String,
        service_name: String,
        line: OutputLine,
    },
    LogStreamStarted {
        app_name: String,
        service_name: String,
        session_id: Uuid,
    },
    LogStreamEnded {
        session_id: Uuid,
        reason: String,
    },

    // New shell messages
    ShellSessionCreated {
        session_id: Uuid,
        app_name: String,
        service_name: String,
    },
    ShellDataReceived {
        session_id: Uuid,
        data: Vec<u8>,  // Raw terminal data with escape sequences
    },
    ShellDataSend {
        session_id: Uuid,
        data: Vec<u8>,  // User input to send to shell
    },
    ShellSessionEnded {
        session_id: Uuid,
        exit_code: Option<i32>,
    },
}
```

## CLI Commands

### app:logs Command

```bash
scottyctl app:logs <app_name> <service> [OPTIONS]

OPTIONS:
    -f, --follow              Follow log output (default: false)
    -n, --lines <NUMBER>      Number of lines to show (default: all available)
        --since <TIMESTAMP>   Show logs since timestamp
        --until <TIMESTAMP>   Show logs until timestamp
    -t, --timestamps          Show timestamps in log output

EXAMPLES:
    scottyctl app:logs my-app web                    # Show all available logs
    scottyctl app:logs my-app web -f                 # Follow logs in real-time
    scottyctl app:logs my-app web -t                 # Show logs with timestamps
    scottyctl app:logs my-app web -n 500             # Show last 500 lines
    scottyctl app:logs my-app web --since 1h         # Logs from last hour
```

### app:shell Command (NOT YET IMPLEMENTED)

**Note**: The backend shell service is fully implemented, but the CLI command is not yet available in scottyctl.

**Planned Command:**
```bash
scottyctl app:shell <app_name> <service> [OPTIONS]

OPTIONS:
    -u, --user <USER>         User to run shell as (default: root)
    -w, --workdir <PATH>      Working directory (default: container default)
        --shell <SHELL>       Shell to use (default: /bin/bash)
        --timeout <DURATION>  Session timeout (default: from config)

PLANNED EXAMPLES:
    scottyctl app:shell my-app web                   # Open bash shell
    scottyctl app:shell my-app web -u www-data       # Shell as www-data user
    scottyctl app:shell my-app db --shell /bin/sh    # Use sh instead of bash
```

**Current Workaround**: Use the REST API directly or wait for CLI implementation (Phase 6).

## Configuration Options

### New Configuration Structure

```yaml
# config/default.yaml additions
output:
  # Task output limits
  max_lines_per_task: 10000           # Max lines to keep per task
  max_line_length: 4096               # Max characters per line
  cleanup_interval: "5m"              # How often to cleanup old output

  # Log streaming limits
  max_log_lines_streaming: 1000       # Max lines for real-time streaming
  log_buffer_size: 10000              # Buffer size for log collection

shell:
  # Shell session management
  default_ttl: "30m"                  # Default session timeout
  max_concurrent_sessions: 10         # Max shells per service
  cleanup_interval: "1m"              # How often to check for expired sessions
  allowed_shells:                     # Allowed shell executables
    - "/bin/bash"
    - "/bin/sh"
    - "/bin/zsh"

websocket:
  # WebSocket specific settings
  max_message_size: 1048576          # 1MB max message size
  ping_interval: "30s"               # Ping interval for connection health
```

### Environment Variable Overrides

```bash
SCOTTY__OUTPUT__MAX_LINES_PER_TASK=5000
SCOTTY__SHELL__DEFAULT_TTL=60m
SCOTTY__OUTPUT__MAX_LOG_LINES_STREAMING=2000
```

## Implementation Status

**Current Status: Backend Complete, Frontend Partial** 🎯

The unified output system backend is fully functional with significant performance and infrastructure improvements. Current state:

**✅ FULLY COMPLETED:**
- **Backend Infrastructure**: All services, APIs, and WebSocket handlers implemented and tested
- **CLI Logs Command**: `app:logs` fully functional with real-time streaming
- **Task Output UI**: Unified output viewer in frontend with WebSocket integration
- **Performance & Reliability**: Deadlock fixes, batching, security improvements
- **Breaking Changes**: Removed stdout/stderr from TaskDetails, WebSocket-only streaming

**🚧 PARTIALLY COMPLETED:**
- **CLI Shell Command**: Backend ready, CLI command NOT implemented
- **Frontend Integration**: Task output viewer done, container logs/shell UI NOT implemented

**Completed Features:**
- ✅ **BREAKING CHANGE IMPLEMENTED**: Removed stdout/stderr from TaskDetails, now uses WebSocket-only task output streaming
- ✅ Full WebSocket-based authenticated log streaming backend with REST API endpoints
- ✅ `app:logs` CLI command with follow, timestamps, line limits, and time filters
- ✅ Interactive shell backend service (ShellService) with WebSocket handlers
- ✅ Real-time task output streaming via TaskOutputData messages with unified-output component
- ✅ WebSocketMessenger architecture for centralized client management
- ✅ Resolved deadlock and lock contention issues (20-100x performance improvement)
- ✅ TimedBuffer batching system for efficient output handling
- ✅ Proper resource cleanup and subscription management
- ✅ Comprehensive test coverage (16 tests) and CI integration
- ✅ Consolidated WebSocket message types in scotty-types crate
- ✅ **Live task output during all app operations** - unified OutputLine format with timestamps and sequence numbers
- ✅ **Build System Optimization** - TypeScript generation optimized from 27s to 6s
- ✅ **Type System Consolidation** - Eliminated all type duplication with scotty-types crate
- ✅ **Frontend Build Optimization** - Migrated to bun for 62% faster builds
- ✅ **Platform-Agnostic Docker Builds** - Multi-platform support with consolidated Rollup binaries
- ✅ **MaskedSecret/SecretHashMap** - Memory-safe secret handling

**Next Steps**:
- Phase 3 Completion: Implement `app:shell` CLI command
- Phase 4 Completion: Add container log viewer and shell UI to frontend

---

## Implementation Plan

### Phase 1: Core Infrastructure ✅ COMPLETED
1. ✅ **Unified Output Data Model**: Implement OutputLine and StreamingTaskOutput structures
2. ✅ **Bollard Integration**: Research and implement bollard container logs and exec APIs
3. ✅ **WebSocket Protocol**: Extend message types for logs and shell data
4. ✅ **Configuration System**: Add new configuration options with validation

### Phase 2: Log Streaming ✅ COMPLETED
1. ✅ **Backend Log API**: Implement REST endpoints and WebSocket handlers for log streaming
2. ✅ **CLI Logs Command**: Implement `app:logs` with WebSocket integration
3. ✅ **Permission Integration**: Add authorization checks for logs access
4. ✅ **Testing**: Unit and integration tests for log streaming

### Phase 3: CLI Commands 🚧 PARTIALLY COMPLETED
1. ✅ **CLI Logs Command**: Fully implemented `app:logs` with WebSocket integration, follow mode, timestamps, line limits
2. ✅ **Permission Integration**: Authorization checks for logs access
3. ✅ **Authentication Enhancement**: Centralized auth system for WebSocket connections
4. ✅ **Stream Cleanup**: Proactive client disconnect cleanup for proper resource management
5. ✅ **User Experience**: Improved completion timing and removed duplicate messages
6. ✅ **Shell Backend**: Complete ShellService implementation with session management, REST/WebSocket APIs
7. ❌ **CLI Shell Command**: NOT IMPLEMENTED - `app:shell` command missing from scottyctl (backend ready)

### Phase 3.5: WebSocket Message Consolidation ✅ COMPLETED
1. ✅ **Message Type Consolidation**: Moved all WebSocket message types to `scotty-core/src/websocket/message.rs`
2. ✅ **Code Deduplication**: Eliminated ~70 lines of duplicate message definitions from scottyctl
3. ✅ **Type Consistency**: Server and client now guaranteed to use identical message types
4. ✅ **Import Updates**: Updated 18 files across server and client to use consolidated types
5. ✅ **Testing**: Verified all tests pass with new consolidated message structure
6. ✅ **Single Source of Truth**: All WebSocket communication types defined once and shared

### Phase 3.6: Task Output WebSocket Streaming ✅ COMPLETED
1. ✅ **Hybrid WebSocket Implementation**: Updated `wait_for_task` function to use REST polling for task status + WebSocket for real-time output
2. ✅ **WebSocketMessenger Architecture**: Created centralized abstraction for WebSocket client management and message broadcasting
3. ✅ **Task Output Display**: Implemented unified output display with colored stderr output during task execution
4. ✅ **Real-time Feedback**: Shows task progress with live stdout/stderr output during app operations
5. ✅ **Stack Overflow Resolution**: Fixed circular reference issues in TaskManager data structures
6. ✅ **Resource Management**: Proper WebSocket subscription cleanup and client management
7. ✅ **Unified Experience**: Consistent streaming experience across logs, shell, and task operations
8. ✅ **Status Integration**: Uses `set_status` for proper UI status updates

### Phase 3.7: Infrastructure Optimization ✅ COMPLETED
1. ✅ **TypeScript Generation Optimization**: Created standalone `ts-generator` crate reducing build time from 27s to 6s
2. ✅ **Type System Consolidation**: Moved all shared types to `scotty-types` crate as single source of truth
3. ✅ **Import Cleanup**: Updated all files to use direct imports from `scotty-types` instead of re-exports
4. ✅ **Frontend Build Migration**: Switched from npm to bun for 62% faster frontend builds (3.2s vs 5.2s)
5. ✅ **Docker Build Optimization**: Implemented platform-agnostic Docker builds with multi-platform Rollup binaries
6. ✅ **Workspace Integration**: Added new crates to workspace for better tooling and dependency management
7. ✅ **Legacy Cleanup**: Removed duplicate dependencies, old package manager files, and unused code
8. ✅ **Multi-Platform Support**: Docker builds now work on ARM64, x86_64, and different libc implementations

### Phase 4: Frontend Integration 🚧 PARTIALLY COMPLETED
1. ✅ **Unified Output Viewer**: Created `unified-output.svelte` component with chronological display
2. ✅ **WebSocket-Only Streaming**: Task output uses WebSocket streaming (TaskOutputData messages)
3. ✅ **Real-time Updates**: Live task output streaming during execution via WebSocket
4. ✅ **Task Output Store**: Implemented taskOutputStore.ts for managing streaming data
5. ✅ **WebSocket Store**: Connection management and message handling in webSocketStore.ts
6. ✅ **Task Detail Page**: Enhanced with real-time output streaming and WebSocket status indicator
7. ❌ **Container Log Viewer UI**: NOT IMPLEMENTED - No frontend UI for viewing service logs
8. ❌ **Interactive Shell UI**: NOT IMPLEMENTED - No xterm.js or shell UI in frontend

### Phase 5: Performance and Reliability ✅ COMPLETED
1. ✅ **Deadlock Resolution**: Fixed critical lock contention causing API hangs (commit 160375a2)
2. ✅ **Performance Optimization**: Reduced write lock frequency 20-100x (1000/sec → 10-50/sec)
3. ✅ **TimedBuffer System**: Generic batching utility with configurable size and time thresholds
4. ✅ **Memory Management**: Proper output limits, cleanup intervals, and resource management
5. ✅ **Lock Pattern Improvements**: Helper methods to ensure consistent, minimal lock holding times
6. ✅ **Security Enhancements**: MaskedSecret and SecretHashMap for memory-safe secret handling
7. ✅ **Error Handling**: Robust error handling with enum-based errors (LogStreamError, ShellServiceError)
8. ✅ **Test Coverage**: 16 comprehensive tests with CI-friendly Docker handling
9. 🚧 **Monitoring**: Metrics and logging for the new system (partial - basic logging in place)
10. 🚧 **Documentation**: User and developer documentation (this PRD complete, end-user docs pending)

### Phase 6: Remaining Features (Future Work)
1. **CLI Shell Command**: Implement `app:shell` command in scottyctl with terminal integration
   - Add ShellCommand struct to cli.rs
   - Implement WebSocket-based shell handler
   - Add TTY resize handling and raw terminal mode
   - Interactive command input/output with proper escape sequences
2. **Frontend Log Viewer**: Add container log viewing UI to web interface
   - Create log viewer component (similar to unified-output.svelte)
   - Add WebSocket handlers for LogLineReceived/LogStreamStarted/LogStreamEnded
   - Add log viewer page or modal for each service
   - Integration with app detail page
3. **Frontend Shell UI**: Add interactive shell terminal to web interface
   - Integrate xterm.js for terminal emulation
   - WebSocket handlers for ShellSession* messages
   - TTY resize support, copy/paste, terminal settings
   - Shell session management UI (list, create, terminate)
4. **Monitoring & Observability**:
   - Add metrics for log/shell session counts, durations, errors
   - OpenTelemetry integration for tracing
   - Performance dashboards
5. **End-User Documentation**:
   - User guide for app:logs command
   - User guide for app:shell command (once implemented)
   - Web UI documentation for log viewer and shell access
   - Security best practices for shell access

## Security Considerations

### Permission Model
- **Logs Permission**: Required for `app:logs` command and log streaming
- **Shell Permission**: Required for `app:shell` command and shell access
- **Scope-based Access**: Permissions checked against app-specific scopes
- **Session Isolation**: Shell sessions are isolated per user and app

### Input Validation
- **Service Name Validation**: Ensure service exists in app's docker-compose
- **Shell Path Validation**: Only allow pre-configured shell executables
- **Command Injection Prevention**: Use bollard APIs directly, avoid shell command construction
- **Rate Limiting**: Implement rate limits for shell session creation

### Data Security
- **Log Data Sanitization**: Option to filter sensitive data from logs
- **Shell Session Logging**: Optional audit logging of shell commands
- **WebSocket Security**: Ensure proper authentication for WebSocket connections
- **TTL Enforcement**: Strict enforcement of session timeouts

## Migration Notes

### Breaking Changes
- `TaskDetails.stdout` and `TaskDetails.stderr` fields will be removed
- Frontend components using separate stdout/stderr displays need updates
- CLI output formatting will change to unified display
- Manual migration required for existing installations

### Migration Strategy
1. **Database/Storage**: No persistent storage migration needed (tasks are ephemeral)
2. **Frontend**: Update components to use new unified output display
3. **CLI**: Update output formatting in scottyctl
4. **API**: Maintain task state endpoints, add new output endpoints

## Testing Strategy

### Unit Tests
- OutputLine serialization/deserialization
- ShellSession management logic
- Configuration validation
- Permission checking logic

### Integration Tests
- Bollard container logs streaming
- Bollard exec session creation and management
- WebSocket message flow
- CLI command functionality

### End-to-End Tests
- Complete log streaming workflow (backend → WebSocket → CLI)
- Complete shell session workflow (CLI → WebSocket → backend → container)
- Permission enforcement across all components
- Frontend integration

## Future Enhancements

### Frontend Shell Terminal
- **xterm.js Integration**: Full terminal emulator in web UI
- **Terminal Sharing**: Multiple users can view same shell session
- **Terminal Recording**: Save and replay shell sessions

### Advanced Log Features
- **Log Search**: Full-text search across historical logs
- **Log Alerts**: Real-time alerts based on log patterns
- **Log Export**: Export logs in various formats (JSON, CSV, etc.)

### Performance Optimizations (Later)
- **Log Compression**: Compress historical logs to save space
- **Streaming Optimization**: Optimized WebSocket streaming for large outputs
- **Caching Layer**: Cache frequently accessed logs

## Next Steps

1. **Review and Approval**: Stakeholder review of this PRD
2. **Technical Spike**: Investigate bollard logs and exec APIs in detail
3. **Architecture Review**: Validate WebSocket protocol design
4. **Implementation Start**: Begin with Phase 1 core infrastructure

## Open Questions

1. **Service Discovery**: How should we map service names to actual container IDs when multiple instances exist?
2. **Log Persistence**: Should we implement any form of log persistence beyond the current task system?
3. **Error Recovery**: How should we handle bollard connection failures and automatic reconnection?