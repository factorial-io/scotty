---
# scotty-5ne9
title: 'Download handler: return proper 404 for missing container paths'
status: completed
type: bug
priority: normal
created_at: 2026-05-29T19:29:45Z
updated_at: 2026-05-29T19:34:36Z
---

app:cp downloads commit 200 OK + chunked encoding before polling the Docker stream, so a 404 (missing path) from Docker aborts the body mid-flight and the client sees an opaque 'error decoding response body'. Fix: peek the first chunk of the download stream before building the response; on Err map via map_bollard_error to a proper 404/409/500 JSON, on Ok stream the buffered chunk + remainder.

## Summary of Changes

Root cause: `download_files_handler` committed `200 OK` + chunked encoding before polling the Docker download stream. Docker validates the container path lazily, so a missing path (or stopped container) only surfaced as a stream error after the status line was sent — the body aborted mid-flight and the client saw an opaque 'error decoding response body' / 'failed to extract downloaded archive'.

Fix (`scotty/src/api/rest/handlers/files.rs`): peek the first chunk of the counted download stream before building the response. On `Err`, map it via `map_bollard_error` and return a structured 404/409/500 JSON error (matching the upload handler). On `Ok`, re-attach the buffered chunk ahead of the remainder via `stream::iter(...).chain(tail)` and stream as before. The size-limit (413) case is unchanged — it is still only detectable mid-stream and aborts the body.

Verified end-to-end against the live server: bad path now returns `[NotFound] Could not find the file ... — check the app name, service name, and container path`; good path downloads successfully. `cargo build` clean, `cargo clippy` clean, 11 `files::` unit tests pass.

Note: the original user report was a wrong container path (`/usr/share/nginx/index.html` vs the actual `/usr/share/nginx/html/index.html`); this fix makes that failure mode legible.
