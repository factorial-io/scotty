## ADDED Requirements

### Requirement: Server SHALL expose REST endpoints for streaming file transfer to and from a service container

The scotty server SHALL provide two authenticated REST endpoints that stream file content between the caller and a container belonging to a service of a scotty-managed app. The endpoints SHALL use Docker's native tar-based copy API (via Bollard) so that file metadata (mode, owner, mtime) is preserved and binary content is transported losslessly.

- `GET  /api/v1/apps/{app}/services/{service}/files?path=<container-path>` SHALL stream a tar archive of `container-path` to the caller (`Content-Type: application/x-tar`).
- `PUT  /api/v1/apps/{app}/services/{service}/files?path=<container-path>` SHALL accept a tar archive in the request body and extract it into `container-path` inside the container.
- Both endpoints SHALL use chunked transfer encoding and SHALL NOT buffer the full payload in memory.

#### Scenario: Download a single file as tar stream
- **WHEN** an authenticated user with `view` permission calls `GET /api/v1/apps/myapp/services/db/files?path=/var/backups/dump.sql`
- **THEN** the server responds `200 OK` with `Content-Type: application/x-tar` and streams a tar archive containing `dump.sql`

#### Scenario: Upload a tar archive into a container directory
- **WHEN** an authenticated user with `manage` permission `PUT`s a tar stream to `/api/v1/apps/myapp/services/web/files?path=/var/www/public`
- **THEN** the server extracts the archive into `/var/www/public` inside the `web` container and responds `204 No Content`

#### Scenario: Unknown app or service
- **WHEN** the caller targets an app or service that does not exist
- **THEN** the server responds `404 Not Found` with an error body identifying which segment was unresolved

#### Scenario: Container not running
- **WHEN** the targeted service has no running container
- **THEN** the server responds `409 Conflict` with an error code `service_not_running`

### Requirement: File transfer endpoints SHALL be authorized via the existing RBAC system

Downloads (`GET`) SHALL require the `view` permission on the app's scope. Uploads (`PUT`) SHALL require the `manage` permission on the app's scope. Authorization SHALL be enforced by the existing Casbin middleware before any Docker call is made.

#### Scenario: Unauthorized download
- **WHEN** a user without `view` permission on the app's scope calls the download endpoint
- **THEN** the server responds `403 Forbidden` and does not contact the Docker daemon

#### Scenario: Read-only user attempts upload
- **WHEN** a user with only `view` permission `PUT`s to the upload endpoint
- **THEN** the server responds `403 Forbidden`

### Requirement: Server SHALL enforce a configurable maximum transfer size

The server SHALL reject uploads whose decoded size exceeds a configurable per-app limit (default 1 GiB) and SHALL abort downloads that exceed the same limit, returning an error to the caller. The limit SHALL be configurable via the `SCOTTY__FILES__MAX_TRANSFER_SIZE` environment variable.

#### Scenario: Upload exceeds size limit
- **WHEN** an upload stream's cumulative byte count exceeds the configured maximum
- **THEN** the server aborts the request and responds `413 Payload Too Large`

### Requirement: Scottyctl SHALL provide an `app:cp` command with docker-cp-compatible syntax

`scottyctl` SHALL add a new subcommand `app:cp <source> <destination>` where each side is either a local path or a remote spec of the form `<app>:<service>:<container-path>`. Exactly one side SHALL be remote. The command SHALL use the server's file transfer endpoints and SHALL produce or consume the tar stream on the client side so that single files, directories, and binary content all work without shell intermediaries.

#### Scenario: Copy a single file from container to local disk
- **WHEN** the user runs `scottyctl app:cp myapp:db:/var/backups/dump.sql ./dump.sql`
- **THEN** the file is downloaded and written to `./dump.sql` with its original mode preserved

#### Scenario: Copy a local directory into a container
- **WHEN** the user runs `scottyctl app:cp ./assets myapp:web:/var/www/public/assets`
- **THEN** the local directory is packed into a tar stream, uploaded, and extracted at the destination path

#### Scenario: Both sides remote or both sides local
- **WHEN** the user provides two remote specs or two local paths
- **THEN** the command exits non-zero with an error explaining that exactly one side must be remote

### Requirement: Scottyctl SHALL treat `-` as stdin/stdout to enable pipe usage

When the local side of `app:cp` is the literal string `-`, `scottyctl` SHALL read from stdin (for uploads) or write to stdout (for downloads) using the same code path as file-to-file transfers. Pipe mode SHALL transport raw bytes, not a tar archive, by packing/unpacking a single synthetic entry on the client.

#### Scenario: Pipe stdout from a database dump into a container
- **WHEN** the user runs `mysqldump db | scottyctl app:cp - myapp:db:/tmp/dump.sql`
- **THEN** scottyctl reads stdin until EOF, wraps it as a tar entry named `dump.sql`, uploads it, and the server extracts it at `/tmp/dump.sql`

#### Scenario: Pipe a remote file to stdout
- **WHEN** the user runs `scottyctl app:cp myapp:web:/var/log/app.log - | gzip > app.log.gz`
- **THEN** scottyctl downloads the tar stream, unpacks the single entry, and writes its bytes to stdout

#### Scenario: Stdin/stdout used with a directory source
- **WHEN** the user pipes to `-` but the remote path resolves to a directory
- **THEN** scottyctl exits non-zero with an error explaining that pipe mode requires a single file

### Requirement: Service name SHALL be resolvable from the app when unambiguous

When the user writes `<app>::<path>` (empty service segment) or `<app>:<path>` (no service segment), `scottyctl` SHALL resolve the service to the app's primary/public service as defined by its blueprint. If no primary service is defined or multiple candidates exist, the command SHALL fail with an error listing available service names.

#### Scenario: Single-service app with implicit service
- **WHEN** the user runs `scottyctl app:cp myapp::/etc/config.yml ./config.yml` against an app whose blueprint defines exactly one public service
- **THEN** the command resolves to that service and succeeds

#### Scenario: Multi-service app with implicit service
- **WHEN** the user runs `scottyctl app:cp myapp::/path ./file` against an app with multiple services and no designated primary
- **THEN** the command exits non-zero with an error listing the available service names

### Requirement: Shared parsing and transfer logic SHALL live in one place

The path-spec parser (`<app>:<service>:<path>` vs. local vs. `-`) and the tar pack/unpack streaming pipeline SHALL be implemented once and reused for both file-mode and pipe-mode operations on the client. The server SHALL likewise route both modes through a single handler per direction (one for download, one for upload).

#### Scenario: Identical server handler powers file and pipe modes
- **WHEN** the client invokes `app:cp` in file mode and in pipe mode against the same destination
- **THEN** both requests hit the same server endpoint and execute the same Bollard call path
