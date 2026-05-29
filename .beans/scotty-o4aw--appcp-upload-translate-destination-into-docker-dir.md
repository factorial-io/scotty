---
# scotty-o4aw
title: 'app:cp upload: translate destination into Docker dir + entry name'
status: completed
type: bug
priority: normal
created_at: 2026-05-29T19:52:46Z
updated_at: 2026-05-29T19:53:00Z
---

Single-file upload failed with 404 because the client sent the full remote destination path as Docker's upload 'path' (which must be an existing directory) and named the tar entry after the local basename, so docker-cp rename semantics never happened.

## Summary of Changes

Client-side fix in scottyctl app:cp upload path (no server change):

- Added split_remote_dest(remote, source_basename) in cp/mod.rs translating the
  destination into (directory, entry_name) the way docker cp does:
  - trailing '/' => copy into that directory, entry keeps the source basename;
  - otherwise => extract into the parent directory and rename the entry to the
    destination basename (so app:cp ./a.pdf app:web:/srv/b.pdf lands as b.pdf).
- upload() now sends the directory as Docker's 'path' query param and passes the
  entry name to the packers. Both file and stdio modes go through this; stdio
  falls back to 'stdin' when no basename is available.
- tar_pack_path and pipe_pack now take an explicit entry_name instead of deriving
  it (pipe_entry_name removed, tests moved to mod.rs as split_remote_dest tests).

Verified against the live server: single-file upload renames to test.pdf;
download roundtrip SHA256 matches the source; trailing-slash dir target keeps the
source basename. 38 cp unit tests pass, clippy clean.
