## 1. Shared types and config

- [x] 1.1 Add `tar = "0.4"` to `scotty/Cargo.toml` and `scottyctl/Cargo.toml` (workspace dep if appropriate)
- [x] 1.2 Add `SCOTTY__FILES__MAX_TRANSFER_SIZE` (default 1 GiB) to settings module with config + env loader
- [x] 1.3 Add shared request/response/error types in `scotty-types` (FileTransferError variants, MAX_TRANSFER_SIZE constant) with ts-rs derives

## 2. Server endpoints

- [x] 2.1 Create `scotty/src/api/rest/handlers/files.rs` with `download_files` and `upload_files` handlers
- [x] 2.2 Implement absolute-path + non-empty validation on the `path` query param
- [x] 2.3 Wire `download_files` to `bollard::Docker::download_from_container`, returning an `axum::body::Body` from the tar stream
- [x] 2.4 Wire `upload_files` to `bollard::Docker::upload_to_container`, consuming the request body as a stream
- [x] 2.5 Implement counting stream wrapper that aborts with `413` when `SCOTTY__FILES__MAX_TRANSFER_SIZE` is exceeded (both directions)
- [x] 2.6 Map "service not running" / "no such container" Bollard errors to `409` with code `service_not_running`
- [x] 2.7 Register routes in `scotty/src/api/router.rs` under `/api/v1/apps/{app}/services/{service}/files` with auth metadata: `view` for GET, `manage` for PUT
- [x] 2.8 Add OpenAPI annotations via utoipa for both endpoints
- [x] 2.9 Integration tests in `scotty/tests/`: download single file, upload + extract, 403 unauthorized, 404 unknown service, 409 not running, 413 size limit

## 3. scottyctl: path-spec parser and shared helpers

- [x] 3.1 Create `scottyctl/src/commands/apps/cp/mod.rs` + submodules (`spec.rs`, `stream.rs`)
- [x] 3.2 Implement `PathSpec` enum and parser with the heuristic (Windows drive letter, existing local path, `-` for stdio)
- [x] 3.3 Implement service resolution helper: fetch app blueprint via existing API client; pick the single `public: true` service or error with candidate list
- [x] 3.4 Implement tar pack helper for local directories/files using `tokio::task::spawn_blocking` + bounded channel adapter
- [x] 3.5 Implement tar unpack helper for download streams using the same blocking-task pattern
- [x] 3.6 Implement pipe-mode pack: wrap stdin into a synthetic single-entry tar with the destination basename
- [x] 3.7 Implement pipe-mode unpack: extract the first regular-file entry to stdout; error if more than one regular-file entry exists
- [x] 3.8 Unit tests for parser (each `PathSpec` case, both-remote/both-local rejection, Windows-drive disambiguation)

## 4. scottyctl: `app:cp` command

- [x] 4.1 Add `CopyCommand` variant to `scottyctl/src/cli.rs` Commands enum with two positional args + help text/examples
- [x] 4.2 Implement `execute` function: parse both args, validate exactly one is `Remote`, dispatch to download or upload flow
- [x] 4.3 Download flow: build URL with encoded path, `reqwest` GET with bearer token, stream body into local tar unpack or stdio unpack
- [x] 4.4 Upload flow: build URL, stream local tar pack or stdio pack as `reqwest` PUT body with `Content-Type: application/x-tar`
- [x] 4.5 Surface server errors (403/404/409/413) as user-friendly stderr messages with non-zero exit code
- [x] 4.6 Implement byte counter that writes to stderr (e.g., `1.2 GiB transferred`) when stderr is a tty

## 5. End-to-end verification

- [ ] 5.1 Manual run: `scottyctl app:cp ./Cargo.toml myapp:web:/tmp/Cargo.toml` and verify file lands with preserved mode
- [ ] 5.2 Manual run: round-trip a binary file (e.g., a PNG) and verify checksum is identical
- [ ] 5.3 Manual run: `cat dump.sql | scottyctl app:cp - myapp:db:/tmp/dump.sql` end-to-end against a real container
- [ ] 5.4 Manual run: `scottyctl app:cp myapp:web:/var/log/app.log - | wc -c` matches container-side `wc -c`
- [ ] 5.5 Manual run: `scottyctl app:cp myapp::/etc/hostname ./hn` against a single-public-service app resolves implicitly
- [ ] 5.6 Manual run: same against a multi-service app fails with a candidate list
- [ ] 5.7 Manual run: upload exceeding `MAX_TRANSFER_SIZE` fails with `413` and a clear message

## 6. Docs and bean cleanup

- [ ] 6.1 Update `docs/content/cli.md` with an `app:cp` section, including pipe-mode examples (matches `scotty-qcv7` scope)
- [ ] 6.2 Update `README.md` quick-start with one `app:cp` example
- [ ] 6.3 Ensure `scottyctl app:cp --help` text is complete (syntax, `-` semantics, implicit service)
- [ ] 6.4 Mark beans: complete `scotty-fad6` epic, complete `scotty-kqlr` and `scotty-qcv7`, scrap `scotty-54nc` with rationale
