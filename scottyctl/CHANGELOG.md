# Changelog

All notable changes to this project will be documented in this file.

## [0.2.8]

### Bug Fixes

- Enable crossterm use-dev-tty feature for shell sessions spawned by parent processes ✔️

## [0.2.7]

### Bug Fixes

- Resolve clippy unnecessary_unwrap warning in app info display ✔️

## [0.2.5]

### Bug Fixes

- Resolve race conditions in task output streaming ✔️

## [0.2.4]

### Bug Fixes

- Remove dates from changelog template to avoid timestamp issues ✔️
- Clear status line before command exits ✔️
- Add Default implementations for StatusLine and Ui to satisfy clippy ✔️
- Validate token in auth:status and return exit code 1 when invalid (GH#607) ✔️
- Address code review feedback for HttpError ✔️
- Prioritize explicit access token over cached OAuth tokens in scottyctl ✔️

### Features

- Preserve HTTP status codes with custom error types ✔️

### Performance

- Optimize compile times by disabling default features and adding telemetry feature flags ✔️

### Refactor

- Improve status line cleanup documentation and consistency ✔️
- Improve status messages for auth commands ✔️
- Improve error handling with custom ApiError type ✔️
- Remove duplicate retry logic from scottyctl ✔️

### Testing

- Add comprehensive test coverage for auth:status token validation ✔️

## [0.2.3]

### Bug Fixes

- Resolve changelog generation issues with empty sections and subshell ✔️
- Skip empty version sections in per-crate changelogs ✔️
- Show error when auth token expired in auth:status ✔️

### Features

- Add domain-based user matching for Casbin RBAC ✔️

### Refactor

- Move Permission enum to scotty-core and iterate over it ✔️

## [0.2.1]

### Refactor

- Standardize to single workspace-level changelog ✔️

## [0.2.0]

### Bug Fixes

- Enable real-time task output streaming in scottyctl ✔️
- Address PR feedback with proper error handling and security ✔️
- Address PR feedback with tests and security improvements ✔️
- Extract and display error messages from API responses ✔️
- Remove non-functional --workdir option from app:shell command ✔️
- Remove double-wrapping of shell commands ✔️
- Resolve critical TTY mode bugs for interactive shell ✔️
- Normalize URLs to prevent double slashes in API calls (#470) ✔️
- Resolve merge conflicts from main branch ✔️
- Update OIDC test data and apply code formatting ✔️
- Scottyctl bearer token authentication with RBAC ✔️

### Documentation

- Document task handle behavior and add WebSocket fallback logging ✔️

### Features

- Add ASCII art logo with version and copyright ✔️
- Add gzip compression for file uploads in app:create ✔️
- Add .scottyignore support for app:create ✔️
- Propagate exit codes in command mode ✔️
- Add terminal size support for interactive shell ✔️
- Implement interactive shell with raw terminal mode ✔️
- Add app:shell command and refactor service validation ✔️
- Add dedicated OutputStreamType variants for status messages ✔️
- Implement real-time task output streaming for Phase 3.6 ✔️
- Improve log command UX and add terminal detection ✔️
- Improve log command options for better UX ✔️
- Simplify --timestamps flag to boolean behavior ✔️
- Implement authenticated WebSocket log streaming with improved UX ✔️
- Refactor TaskDetails and TaskManager to use unified output ✔️
- Implement shared admin types and enhance authentication logging ✔️
- Unify OAuth error handling system and fix device flow polling ✔️
- Consolidate shared functionality and improve OAuth error handling ✔️
- Implement version compatibility check between scottyctl and server ✔️
- Implement complete OAuth device flow for scottyctl ✔️
- Refactor OAuth to OIDC-compliant provider-agnostic system with Gravatar support ✔️
- Implement OAuth session exchange for secure frontend authentication ✔️
- Implement comprehensive OAuth authentication system ✔️

### Performance

- Implement token caching to reduce filesystem access ✔️

### Refactor

- Improve log output styling, performance, and controls ✔️
- Embed TaskOutput directly in TaskDetails for tight coupling ✔️
- Optimize build system and eliminate type duplication ✔️
- Improve messaging consistency and error handling ✔️
- Consolidate message types in scotty-core ✔️
- Reorganize app commands into modular structure and add app:logs command ✔️
- Streamline admin CLI command error handling ✔️
- Remove emojis from admin command success messages ✔️
- Update auth commands to use UI helper and reduce emoji usage ✔️

### Styling

- Apply cargo fmt formatting ✔️

### Testing

- Add comprehensive unit tests for shell feature ✔️

## [0.1.0-alpha.38]

### Bug Fixes

- Normalize URLs to prevent double slashes in API calls (#470) ✔️
- Correct method calls for table column modification ✔️
- Correct function call for formatting services commands ✔️
- Rename format_services_command to format_services_commands for clarity ✔️
- Fix iteration and formatting issues in blueprint lifecycle actions ✔️

### Features

- Add Traefik middleware support and examples ✔️
- Restructure blueprint actions with unified description format ✔️
- Add blueprint:info command and action descriptions ✔️
- Add support for custom actions on apps ✔️

### Refactor

- Migrate environment variables to SecretHashMap ✔️
- Simplify lifecycle action handling ✔️

### Styling

- Apply new Rust format string syntax ✔️

## [0.1.0-alpha.30]

### Bug Fixes

- Make AppContext fields private with getter methods ✔️
- Replace atty dependency with std::io::IsTerminal ✔️

### Features

- Refactor to use shared AppContext with unified UI ✔️
- Add retry mechanism with backoff for API calls ✔️

## [0.1.0-alpha.29]

### Bug Fixes

- Remove trailing newlines from UI messages ✔️
- Fix: Change task output from stderr to stdout if it was targeted to
stdout ❌
- Include file path in env file parse error message ✔️
- Fix environment variable precedence in app creation ✔️
- Support binary file handling in file reading ✔️

### Features

- Enhance user interface with status line functionality ✔️
- Enhance status line with emoji indicators ✔️

### Refactor

- Modularize and reorganize file and parser utilities ✔️
- Introduce StatusLine for better status tracking and UI feedback ✔️

## [0.1.0-alpha.23]

### Refactor

- Improve env var parsing logic ✔️

## [0.1.0-alpha.22]

### Features

- Add new flag destroy_on_ttl which lets you destroy an app instead of stopping it after the TTL expired. ✔️
- Add dotenv integration and restructure scottyctl command handling ✔️

## [0.1.0-alpha.16]

### Documentation

- Add readmes for all three apps/libs ✔️

### Features

- Restructure into workspaces (#152) ✔️

