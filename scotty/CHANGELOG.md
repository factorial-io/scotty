# Changelog

All notable changes to this project will be documented in this file.

## [0.2.2] - 2025-11-28

### Bug Fixes

- Skip empty version sections in per-crate changelogs ✔️

### Documentation

- Add comprehensive documentation for domain-based assignments ✔️

### Features

- Add domain-based user matching for Casbin RBAC ✔️

### Refactor

- Use Casbin user_match_impl as single source of truth ✔️
- Use Casbin user_match_impl as single source of truth ✔️
- Move Permission enum to scotty-core and iterate over it ✔️

## [0.2.1] - 2025-11-28

### Refactor

- Standardize to single workspace-level changelog ✔️

## [0.2.0] - 2025-11-28

### Bug Fixes

- Implement case-insensitive email matching per RFC 5321 ✔️
- Add allow dead_code attribute to validate_domain_assignment ✔️
- Enable real-time task output streaming in scottyctl ✔️
- Enable real-time task output streaming in scottyctl ✔️
- Improve decompression error handling and size limit enforcement ✔️
- Address PR feedback with proper error handling and security ✔️
- Propagate errors from handlers to tasks ✔️
- Prevent secrets in Docker images and clarify identifier vs token distinction ✔️
- Fix broken doctests after adding lib.rs ✔️
- Suppress dead_code warnings for test utils ✔️
- Resolve clippy warnings for pre-push hook ✔️
- Resolve critical TTY mode bugs for interactive shell ✔️
- Add .env file loading support for server configuration ✔️
- Use singleton ShellService across all handlers ✔️
- Add handler for ShellSessionData input ✔️
- Address critical rate limiting issues from PR review ✔️
- Add IP headers to rate limiting integration tests ✔️
- Protect PKCE verifier and CSRF token with MaskedSecret ✔️
- Replace hardcoded localhost with configurable frontend base URL ✔️
- Apply constant-time comparison to login handler ✔️
- Use constant-time comparison for bearer token validation ✔️
- Apply constant-time comparison to login handler ✔️
- Use constant-time comparison for bearer token validation ✔️
- Replace hardcoded localhost with configurable frontend base URL ✔️
- Resolve clippy linting errors in metrics modules ✔️
- Enable HTTP metrics middleware when metrics telemetry is enabled ✔️
- Resolve WebSocket dev mode authentication and security issues ✔️
- Resolve deadlock and lock contention in task management ✔️
- Resolve merge conflicts from main branch ✔️
- Resolve wildcard scope expansion bug in authorization system ✔️
- Update secure_response_test for removed TaskDetails fields ✔️
- Show container status messages to clients via task output ✔️
- Fix code warning ✔️
- Improve bearer token authentication and error logging ✔️
- Align Casbin model matcher between test and production environments ✔️
- Improve authorization security and robustness ✔️
- Resolve clippy warnings and improve code quality ✔️
- Update OIDC test data and apply code formatting ✔️
- Centralize user ID logic and fix bearer token authorization ✔️
- Resolve RBAC authorization middleware issues ✔️
- Remove unnecessary assert!(true) statements flagged by clippy ✔️
- Resolve clippy warnings and format code ✔️

### Documentation

- Document task handle behavior and add WebSocket fallback logging ✔️
- Address PR review feedback for rate limiting ✔️
- Add comprehensive rate limiting documentation ✔️

### Features

- Add ASCII art logo with version and copyright ✔️
- Add validation, tests, and documentation for domain assignments ✔️
- Add domain-based role assignment support ✔️
- Add gzip compression for file uploads in app:create ✔️
- Support bearer token fallback when OAuth is enabled ✔️
- Add structured audit logging for compliance ✔️
- Add comprehensive rate limiting metrics ✔️
- Add rate limiting metrics and Grafana dashboard ✔️
- Implement comprehensive API rate limiting ✔️
- Implement session cleanup and comprehensive monitoring ✔️
- Instrument shell service with metrics (scotty-10) ✔️
- Add task output streaming metrics (scotty-16) ✔️
- Add WebSocket metrics instrumentation (scotty-11) ✔️
- Add AppList metrics for application monitoring (scotty-20) ✔️
- Upgrade to OpenTelemetry 0.31 and implement custom HTTP metrics ✔️
- Enhance metrics collection and add HTTP metrics infrastructure ✔️
- Add stable Tokio task metrics tracking ✔️
- Add task metrics and refactor to use dedicated helper functions ✔️
- Add memory usage metrics (scotty-17) ✔️
- Make OTLP endpoint configurable via environment variable ✔️
- Instrument log streaming service with OpenTelemetry metrics ✔️
- Add OpenTelemetry metrics module with ScottyMetrics struct ✔️
- Add dedicated OutputStreamType variants for status messages ✔️
- Implement real-time task output streaming for Phase 3.6 ✔️
- Implement unified task output streaming system ✔️
- Improve log command options for better UX ✔️
- Implement authenticated WebSocket log streaming with improved UX ✔️
- Integrate logs and shell endpoints into API router ✔️
- Implement logs and shell API endpoints ✔️
- Integrate service errors with AppError ✔️
- Implement bollard log streaming and shell services ✔️
- Add WebSocket message types for logs and shell sessions ✔️
- Refactor TaskDetails and TaskManager to use unified output ✔️
- Add unified output system and configuration ✔️
- Add bollard technical spike and findings documentation ✔️
- Implement OIDC profile picture support in user avatars ✔️
- Enhance OIDC user info capture and logging ✔️
- Use email addresses as user identifiers for OAuth users ✔️
- Implement shared admin types and enhance authentication logging ✔️
- Add comprehensive admin API for authorization management ✔️
- Implement comprehensive RBAC authorization system ✔️
- Unify OAuth error handling system and fix device flow polling ✔️
- Consolidate shared functionality and improve OAuth error handling ✔️
- Implement version compatibility check between scottyctl and server ✔️
- Add comprehensive authentication testing for scotty backend ✔️
- Implement complete OAuth device flow for scottyctl ✔️
- Refactor OAuth to OIDC-compliant provider-agnostic system with Gravatar support ✔️
- Implement OAuth session exchange for secure frontend authentication ✔️
- Improve OAuth login flow and authentication validation ✔️
- Implement comprehensive OAuth authentication system ✔️
- Implement OAuth authentication system with hybrid auth modes ✔️

### Performance

- Check bearer tokens before OAuth to avoid network latency ✔️

### Refactor

- Extract task completion logic into shared helper ✔️
- Streamline bearer token check and improve logging context ✔️
- Add SessionGuard for panic-safe cleanup ✔️
- Migrate from REST to WebSocket-only implementation ✔️
- Remove unnecessary base64 encoding from PKCE verifier ✔️
- Update authorization config to use serde_norway ✔️
- Use spawn_instrumented for consistent Tokio metrics tracking ✔️
- Improve log output styling, performance, and controls ✔️
- Unify task completion handlers and fix state management ✔️
- Embed TaskOutput directly in TaskDetails for tight coupling ✔️
- Reduce app state creation duplication in bearer_auth_tests ✔️
- Optimize build system and eliminate type duplication ✔️
- Improve messaging consistency and error handling ✔️
- Consolidate message types in scotty-core ✔️
- Restructure handlers into REST and WebSocket modules ✔️
- Improve error handling and add helper methods ✔️
- Remove unused get_user_by_token method from AuthorizationService ✔️
- Replace authorization groups terminology with scopes ✔️
- Make RBAC configuration mandatory and improve logging ✔️

### Security

- Fix domain extraction to prevent bypass via multiple @ symbols ✔️

### Styling

- Apply cargo fmt and fix clippy warnings ✔️
- Apply cargo fmt formatting ✔️
- Apply cargo fmt formatting fixes ✔️

### Testing

- Add E2E WebSocket integration tests ✔️
- Add comprehensive unit tests for shell feature ✔️
- Add comprehensive tests for logs and shell services ✔️

## [0.1.0] - 2025-11-28

### Bug Fixes

- Improve error reporting and fix env vars in custom actions ✔️

## [0.1.0-alpha.38] - 2025-11-28

### Bug Fixes

- Address code review feedback ✔️
- Apply environment variables to all containers, not only public services ✔️
- Remove unused CustomActionResponse struct ✔️
- Update usage of InspectContainerOptions for compatibility ✔️

### Features

- Migrate core secrets to MaskedSecret (Phase 1) ✔️
- Implement MaskedSecret and SecretHashMap for memory-safe secret handling ✔️
- Replace serde_yml with serde_norway dependency ✔️
- Add Traefik middleware support and examples ✔️
- Add notification type for completed custom app actions ✔️
- Restructure blueprint actions with unified description format ✔️
- Add support for custom actions on apps ✔️

### Refactor

- Migrate environment variables to SecretHashMap ✔️
- Update import path for InspectContainerOptions ✔️

### Styling

- Apply new Rust format string syntax ✔️

## [0.1.0-alpha.33] - 2025-11-28

### Bug Fixes

- Add SecureJson wrapper to mask sensitive env vars in API responses ✔️

## [0.1.0-alpha.32] - 2025-11-28

### Bug Fixes

- Reduce lock scope in wait_for_all_containers_handler ✔️
- Remove duplicate WaitForAllContainers handler ✔️
- Add container readiness check and improve Drush commands ✔️

### Features

- Wait for containers to be ready before running post-actions ✔️

## [0.1.0-alpha.30] - 2025-11-28

### Features

- Expose public URLs as environment variables to actions ✔️

## [0.1.0-alpha.29] - 2025-11-28

### Bug Fixes

- Enhance error messages for root folder path resolution ✔️

### Features

- Enhance user interface with status line functionality ✔️
- Embed frontend files into the executable ✔️

### Refactor

- Streamline router setup for improved clarity ✔️
- Upgrade axum to 0.8.1 ✔️
- Improve builder pattern for configuration loading ✔️

## [0.1.0-alpha.25] - 2025-11-28

### Refactor

- Change AppSettings from_file to return Option ✔️

### Testing

- Add tests for handling edge cases in environment variable parsing ✔️

## [0.1.0-alpha.24] - 2025-11-28

### Bug Fixes

- Update security scheme to bearerAuth ✔️
- Improve error handling for missing environment variables ✔️

### Documentation

- Document bearer token authentication with utoipa ✔️

### Features

- Display application last checked timestamp ✔️
- Add redaction for sensitive environment variables ✔️
- Add environment variable substitution functionality ✔️
- Enhance Docker Compose validation for environment variables ✔️
- Add support for environment variables in docker-compose validation ✔️

### Refactor

- Improve environment variable checking in Docker Compose validation ✔️
- Simplify environment variable checks and service validation ✔️
- Simplify environment variable processing ✔️
- Streamline regex initialization in environment variable processing ✔️
- Enhance docker-compose command execution with better error handling and documentation ✔️

### Styling

- Format code for consistency and readability ✔️

## [0.1.0-alpha.22] - 2025-11-28

### Bug Fixes

- Fix crash wehn blueprint cant be found, return proper error now ✔️

### Features

- Add new flag destroy_on_ttl which lets you destroy an app instead of stopping it after the TTL expired. ✔️

## [0.1.0-alpha.21] - 2025-11-28

### Bug Fixes

- Fix middleware setup for traefik config and multiple domains #194 ✔️

## [0.1.0-alpha.20] - 2025-11-28

### Bug Fixes

- Log into registry before trying to run docker run ✔️
- Apply clippy fixes ✔️

### Testing

- Add a test for haproxy and custom domains ✔️

## [0.1.0-alpha.19] - 2025-11-28

### Bug Fixes

- Fork slugify to support up to two dashes as separator ✔️

## [0.1.0-alpha.18] - 2025-11-28

### Bug Fixes

- Slugify app-names passed to the API -- (Fixes #158) ✔️

## [0.1.0-alpha.17] - 2025-11-28

### Bug Fixes

- Fix for wrong traefik config regarding TLS (Fixes #157) ✔️

## [0.1.0-alpha.16] - 2025-11-28

### Documentation

- Add readmes for all three apps/libs ✔️

### Features

- Restructure into workspaces (#152) ✔️

