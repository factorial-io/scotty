## Why

Operators of scotty-hosted applications routinely need to move files between their workstation and a running container (database dumps, log bundles, config files, asset uploads, on-the-fly backups). Today the only path is `app:shell`, which is interactive and cannot reliably transport binary content. A first-class file transfer mechanism — usable both as an explicit `cp` command and as a Unix-style pipe — unblocks critical day-2 workflows (DB import/export, log collection, asset deployment) without forcing users to install `kubectl`/`docker` against the host or invent ad-hoc tar pipelines through the shell endpoint.

## What Changes

- Add a new `app:cp` command to `scottyctl` modeled after `docker cp`, with syntax `app:cp <src> <dst>` where either side may be `<app>:<service>:<path>` or a local path.
- Treat `-` as a sentinel for stdin/stdout, so the same command transparently supports pipes: `mysqldump … | scottyctl app:cp - myapp:db:/tmp/dump.sql` and `scottyctl app:cp myapp:db:/var/log/app.log - | less`.
- When a service name is omitted (`<app>::<path>` or `<app>:<path>`), resolve to the app's primary/public service from the blueprint; require an explicit service when ambiguous and emit a clear error listing candidates.
- Add scotty server REST endpoints that wrap Bollard's `download_from_container` / `upload_to_container` (tar streaming) so the same server-side handler powers both file-to-file and pipe modes.
- Enforce existing RBAC: file transfer requires `manage` (write) or a new `files` permission scoped per app; downloads require `view`. (Decision deferred to design.)
- Gate large/streaming bodies through the existing rate limiter and per-app size limits (configurable).
- Frontend: out of scope for this change (CLI + server only).

## Capabilities

### New Capabilities
- `app-file-transfer`: Streaming file transfer between local filesystem (or stdio) and a service container inside a scotty-managed application, via HTTP tar streams and a `scottyctl app:cp` command.

### Modified Capabilities
<!-- None: no existing specs in openspec/specs/. -->

## Impact

- **scotty** (server): new module `scotty/src/api/rest/handlers/files.rs`; new routes under `/api/v1/apps/{app}/services/{service}/files`; uses existing Bollard `Docker` client and authorization middleware. New config keys for max transfer size and timeout.
- **scottyctl**: new `app:cp` subcommand under `scottyctl/src/commands/apps/cp.rs`; new CLI variant in `cli.rs`; shared path-spec parser. Uses existing auth/preflight machinery.
- **scotty-core / scotty-types**: shared request/response types for the file transfer endpoints (path validation, error variants), exported via ts-rs.
- **Dependencies**: add `tar = "0.4"` (already considered in beans `scotty-kqlr`). No new auth deps.
- **Docs**: README + `docs/content/cli.md` examples (tracked separately by bean `scotty-qcv7`).
- **Beans context**: supersedes epic `scotty-fad6`. The two-phase split (WebSocket binary pipe in `scotty-54nc` vs. dedicated `app:cp` in `scotty-kqlr`) is collapsed into a single unified design that uses the Docker copy API for both file and pipe modes — simpler, no WebSocket binary frame protocol needed.
