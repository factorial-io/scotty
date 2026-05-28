## Context

Scotty currently exposes interactive shell and log streaming over WebSocket (`scotty/src/api/websocket/`) and uses Bollard to talk to the Docker daemon. There is no first-class way to move files between an operator's workstation and a container; users today resort to `app:shell` with `cat`/`tar` tricks, which fails on binary content and tty-coupled stdio. Bean epic `scotty-fad6` and its children (`scotty-54nc`, `scotty-kqlr`, `scotty-qcv7`) captured the two original approaches; this change unifies them.

The relevant existing infrastructure:
- `bollard::Docker::download_from_container` and `upload_to_container` already speak tar over HTTP to the Docker daemon. We re-stream that.
- Axum + tower handlers already exist under `scotty/src/api/rest/handlers/`. Authorization is enforced by `scotty/src/api/middleware/` Casbin layer.
- `scottyctl` already has preflight/auth/server-URL plumbing (`scottyctl/src/preflight.rs`, `auth/`); subcommands live under `scottyctl/src/commands/`.

## Goals / Non-Goals

**Goals:**
- One server handler per direction (download, upload) shared by file mode and pipe mode.
- `scottyctl app:cp` that mirrors `docker cp` ergonomics, including `-` for stdin/stdout.
- Preserve file metadata (mode, owner, mtime) for file-mode transfers via tar.
- Bounded memory: never buffer the full transfer on either side.
- Enforce existing RBAC; no new auth surface.

**Non-Goals:**
- Resumable transfers, range requests, checksums (can be a follow-up).
- Progress bars on the server side (client can render its own from byte counts).
- Frontend UI for file transfer (CLI only).
- Symlink/special-file handling beyond what Bollard's tar layer already supports.
- Replacing the WebSocket shell endpoint or adding a binary stdin frame to it (the previously planned Phase 1, bean `scotty-54nc`, is now superseded — `app:cp - …` covers that workflow).

## Decisions

### D1. Transport: HTTP chunked + tar, not WebSocket

We use HTTP `GET`/`PUT` with `application/x-tar` bodies and chunked transfer encoding.

- Rationale: docker and kubernetes both went this route for `cp`. We're already producing/consuming tar (because Bollard does), TCP gives us free backpressure, and we avoid defining a custom binary frame format. WebSocket would force us to invent an offset/EOF/error envelope just to carry bulk bytes — value-add zero for a unidirectional transfer.
- Alternatives considered:
  - WebSocket binary frames (consistent with shell/logs). Rejected: WS is justified for interactive bidirectional sessions, not bulk one-shot transfers. The "consistency" win is real but the framing-protocol cost is larger than the saved auth-plumbing.
  - Multipart upload. Rejected: tar already does what multipart would, with metadata for free.

### D2. Endpoints

```
GET  /api/v1/apps/{app}/services/{service}/files?path=<container-path>
PUT  /api/v1/apps/{app}/services/{service}/files?path=<container-path>
```

- `path` is required and URL-encoded. Server validates it is absolute and non-empty before any Docker call.
- `GET` streams `application/x-tar` (whatever Bollard returns from `download_from_container`).
- `PUT` accepts `application/x-tar`; body is passed through to `upload_to_container`.
- Responses: `200`/`204` on success; `403` unauthorized; `404` unknown app/service/path; `409` service not running; `413` size limit exceeded; `500` Docker error (with sanitized message).

Service segment can be implicit on the client (see D4); on the wire the server always sees an explicit `service`.

### D3. Shared server handler

A single `scotty/src/api/rest/handlers/files.rs` exposes two handlers (`download_files`, `upload_files`). Both look up app+service via existing `AppState` helpers, run the auth check (already done by middleware via route metadata), and call Bollard. The Bollard byte stream is piped to/from the HTTP body using `axum::body::Body::from_stream` (download) and `request.into_body().into_data_stream()` (upload). A counting wrapper enforces `SCOTTY__FILES__MAX_TRANSFER_SIZE` and aborts the stream with `413` once exceeded.

This satisfies the spec's "one handler per direction" requirement and means file-mode vs. pipe-mode is purely a client concern.

### D4. Client path-spec parser

A standalone module `scottyctl/src/commands/apps/cp/spec.rs` parses each argument into one of:

```rust
enum PathSpec {
    Local(PathBuf),
    Stdio,                                       // literal "-"
    Remote { app: String, service: Option<String>, path: String },
}
```

- Argument is `Remote` iff it contains a `:` and the part before the first `:` is not a Windows drive letter and not an existing local path. (Same heuristic docker uses.)
- `app:service:path` → both filled. `app::path` or `app:path` → `service = None` (server resolution can't help; we resolve client-side from the blueprint via `GET /api/v1/authenticated/apps/{app}` to keep the server endpoint simple).
- Pre-flight: exactly one side must be `Remote`. The other side may be `Local` or `Stdio`.

### D5. Implicit service resolution

When `service` is `None`, scottyctl fetches the app's blueprint info and picks:
1. The service marked `public: true` in the blueprint, if exactly one exists.
2. Otherwise, error with the list of service names. No "first alphabetically" guesswork — explicit failure is better than surprising the user.

Rationale: the blueprint already encodes `required` vs `public` services; reusing that signal avoids inventing a new "primary" attribute.

### D6. Pipe mode wire format

In pipe mode, the body on the wire is still a tar archive containing one regular-file entry. The client synthesizes the entry (upload) or extracts the first regular-file entry (download).

- Upload entry name: derived from the basename of the *remote* destination path (so `app:cp - app:svc:/tmp/dump.sql` becomes a tar entry `dump.sql` extracted into `/tmp/`).
- Download: if the tar contains more than one regular-file entry (i.e., the path was a directory), abort with a clear error — "pipe mode requires a single file". This is symmetrical with `docker cp dir -` behavior.
- File metadata for pipe entries: mode `0644`, current mtime, no owner. Pipe mode is by definition lossy w.r.t. metadata.

### D7. Streaming on the client

The client uses `reqwest` with `body(Body::wrap_stream(...))` (upload) and `bytes_stream()` (download). The tar pack/unpack uses the synchronous `tar` crate driven from a blocking task (`tokio::task::spawn_blocking`) wrapped with channel adapters — `tar` is the de-facto Rust crate but is sync; doing this once in a helper avoids spreading the pattern.

### D8. Bean alignment

- Mark `scotty-fad6` (epic) and `scotty-kqlr` (Phase 2) as superseded by this OpenSpec change once /opsx:apply starts.
- `scotty-54nc` (WebSocket binary stdin for shell) is scrapped: the use case it targeted is fully covered by `app:cp - …` and we are explicitly choosing not to add a binary stdin frame to the shell endpoint.
- `scotty-qcv7` (docs) remains, will be picked up in this change's tasks.

## Risks / Trade-offs

- **Tar crate is sync** → Mitigation: isolate in `spawn_blocking` + bounded channels; only one helper module touches it.
- **No resume / no checksum** in v1 → Mitigation: callout in docs; followup bean for large-file resilience.
- **Implicit service resolution adds a round-trip** for the blueprint lookup → Mitigation: only triggered when service is omitted; small payload; cache for the duration of the command.
- **Tar with absolute / parent-traversal entries on upload** could escape the destination if Docker honored them → Mitigation: client refuses to pack non-relative entry names; server relies on Bollard/Docker which already sandboxes extraction.
- **Operators may expect docker-cp's "merge into existing dir" semantics** (`docker cp foo/. container:/dst/` vs `docker cp foo container:/dst/`) → Mitigation: document that we follow Bollard/Docker semantics verbatim; do not invent our own.
- **Large transfers may hit reverse-proxy timeouts** in self-hosted setups → Mitigation: document a recommended `proxy_read_timeout` / equivalent; configurable server-side request timeout that is generous by default.

## Migration Plan

- Backwards compatible. New endpoints, new CLI subcommand; nothing existing changes.
- Roll out behind a feature flag? No — the endpoints are gated by RBAC, and the CLI subcommand only activates when invoked.
- Rollback: revert the merge; no data migration.

## Open Questions

- Should we expose progress via a trailer header (`X-Bytes-Transferred`) or only via client-side counting? *Lean: client-side only for v1.*
- Do we need a separate `files` permission, or is `view`/`manage` sufficient? *Lean: reuse existing perms for v1; revisit if customers want finer-grained control.*
- `SCOTTY__FILES__MAX_TRANSFER_SIZE` default — 1 GiB feels right but should be confirmed by a deployment owner.
