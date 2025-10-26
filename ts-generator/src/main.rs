/*!
 * Standalone TypeScript type generator for Scotty WebSocket messages
 *
 * This is an optimized, minimal binary that only compiles the types needed
 * for TypeScript generation, significantly reducing compile time compared
 * to building the full workspace.
 *
 * Usage: cargo run (from ts-generator directory)
 */

use scotty_types::{ts_rs::TS, *};

fn generate_index_file(export_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let index_path = export_dir.join("index.ts");
    let content = r#"// This file provides type guards for WebSocket messages
// Auto-generated type guards based on WebSocketMessage union type

import type { WebSocketMessage } from './WebSocketMessage';

// Type guards for task output messages
export function isTaskOutputStreamStarted(
	msg: WebSocketMessage
): msg is { type: 'TaskOutputStreamStarted'; data: { task_id: string; total_lines: bigint } } {
	return msg.type === 'TaskOutputStreamStarted';
}

export function isTaskOutputData(
	msg: WebSocketMessage
): msg is { type: 'TaskOutputData'; data: import('./TaskOutputData').TaskOutputData } {
	return msg.type === 'TaskOutputData';
}

export function isTaskOutputStreamEnded(
	msg: WebSocketMessage
): msg is { type: 'TaskOutputStreamEnded'; data: { task_id: string; reason: string } } {
	return msg.type === 'TaskOutputStreamEnded';
}

// Type guards for log stream messages
export function isLogsStreamStarted(
	msg: WebSocketMessage
): msg is { type: 'LogsStreamStarted'; data: import('./LogsStreamInfo').LogsStreamInfo } {
	return msg.type === 'LogsStreamStarted';
}

export function isLogsStreamData(
	msg: WebSocketMessage
): msg is { type: 'LogsStreamData'; data: import('./LogsStreamData').LogsStreamData } {
	return msg.type === 'LogsStreamData';
}

export function isLogsStreamEnded(
	msg: WebSocketMessage
): msg is { type: 'LogsStreamEnded'; data: import('./LogsStreamEnd').LogsStreamEnd } {
	return msg.type === 'LogsStreamEnded';
}

export function isLogsStreamError(
	msg: WebSocketMessage
): msg is { type: 'LogsStreamError'; data: import('./LogsStreamError').LogsStreamError } {
	return msg.type === 'LogsStreamError';
}

// Type guards for shell session messages
export function isShellSessionCreated(
	msg: WebSocketMessage
): msg is { type: 'ShellSessionCreated'; data: import('./ShellSessionInfo').ShellSessionInfo } {
	return msg.type === 'ShellSessionCreated';
}

export function isShellSessionData(
	msg: WebSocketMessage
): msg is { type: 'ShellSessionData'; data: import('./ShellSessionData').ShellSessionData } {
	return msg.type === 'ShellSessionData';
}

export function isShellSessionEnded(
	msg: WebSocketMessage
): msg is { type: 'ShellSessionEnded'; data: import('./ShellSessionEnd').ShellSessionEnd } {
	return msg.type === 'ShellSessionEnded';
}

export function isShellSessionError(
	msg: WebSocketMessage
): msg is { type: 'ShellSessionError'; data: import('./ShellSessionError').ShellSessionError } {
	return msg.type === 'ShellSessionError';
}

// Type guards for task info messages
export function isTaskInfoUpdated(
	msg: WebSocketMessage
): msg is { type: 'TaskInfoUpdated'; data: import('./TaskDetails').TaskDetails } {
	return msg.type === 'TaskInfoUpdated';
}

// Type guards for app messages
export function isAppListUpdated(msg: WebSocketMessage): msg is { type: 'AppListUpdated' } {
	return msg.type === 'AppListUpdated';
}

export function isAppInfoUpdated(
	msg: WebSocketMessage
): msg is { type: 'AppInfoUpdated'; data: string } {
	return msg.type === 'AppInfoUpdated';
}

// Type guards for authentication messages
export function isAuthenticationSuccess(
	msg: WebSocketMessage
): msg is { type: 'AuthenticationSuccess' } {
	return msg.type === 'AuthenticationSuccess';
}

export function isAuthenticationFailed(
	msg: WebSocketMessage
): msg is { type: 'AuthenticationFailed'; data: { reason: string } } {
	return msg.type === 'AuthenticationFailed';
}

// Type guards for error messages
export function isError(msg: WebSocketMessage): msg is { type: 'Error'; data: string } {
	return msg.type === 'Error';
}

// Type guards for ping/pong
export function isPing(msg: WebSocketMessage): msg is { type: 'Ping' } {
	return msg.type === 'Ping';
}

export function isPong(msg: WebSocketMessage): msg is { type: 'Pong' } {
	return msg.type === 'Pong';
}

// Re-export all types
export * from './WebSocketMessage';
export * from './TaskOutputData';
export * from './OutputLine';
export * from './OutputStreamType';
export * from './TaskDetails';
export * from './LogStreamRequest';
export * from './LogsStreamData';
export * from './LogsStreamEnd';
export * from './LogsStreamError';
export * from './LogsStreamInfo';
export * from './ShellSessionData';
export * from './ShellSessionEnd';
export * from './ShellSessionError';
export * from './ShellSessionInfo';
export * from './ShellDataType';
export * from './State';
"#;

    std::fs::write(&index_path, content)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build absolute path to frontend generated types folder
    // CARGO_MANIFEST_DIR points to the ts-generator directory, parent is workspace root
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = std::path::Path::new(&manifest_dir)
        .parent()
        .expect("Failed to get workspace root");
    let export_dir = workspace_root.join("frontend/src/generated");

    // Set the export directory for ts-rs
    std::env::set_var("TS_RS_EXPORT_DIR", export_dir.to_str().unwrap());

    println!("🔧 Generating TypeScript bindings for WebSocket messages...");

    // Core output types
    OutputStreamType::export()?;
    OutputLine::export()?;

    // Task types
    State::export()?;
    TaskDetails::export()?;
    TaskOutputData::export()?;

    // Log streaming types
    LogStreamRequest::export()?;
    LogsStreamInfo::export()?;
    LogsStreamData::export()?;
    LogsStreamEnd::export()?;
    LogsStreamError::export()?;

    // Shell session types
    ShellDataType::export()?;
    ShellSessionInfo::export()?;
    ShellSessionData::export()?;
    ShellSessionEnd::export()?;
    ShellSessionError::export()?;

    // WebSocket message types
    WebSocketMessage::export()?;

    // Generate index.ts barrel file with type guards and re-exports
    generate_index_file(&export_dir)?;

    println!("✅ TypeScript bindings generated successfully!");
    println!("📁 Generated files location: {}", export_dir.display());

    // List generated files
    let generated_dir = &export_dir;
    if generated_dir.exists() {
        println!("\n📋 Generated files:");
        let mut entries: Vec<_> = std::fs::read_dir(generated_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "ts")
                    .unwrap_or(false)
            })
            .collect();

        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            println!("   - {}", entry.file_name().to_string_lossy());
        }
    }

    Ok(())
}
