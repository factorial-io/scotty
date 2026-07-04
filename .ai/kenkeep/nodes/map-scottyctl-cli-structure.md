---
type: map
title: scottyctl CLI namespace and behavior
description: >-
  Colon-namespaced commands, global flags, version preflight check, and
  gzip+base64 file upload with .scottyignore.
tags:
  - cli
  - scottyctl
  - map
kk_schema_version: 3
kk_id: map-scottyctl-cli-structure
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
scottyctl commands use a colon-separated namespace: `app:` (list, create, destroy, run, start, stop, rebuild, purge, adopt, info, action, logs, shell), `admin:` (scopes/roles/assignments/permissions), `auth:` (login, logout, status, refresh), `blueprint:` (list, info), `notify:` (add, remove), plus `completion` and `test`. Global flags: `--server`, `--access-token`, `--debug`, `--bypass-version-check`.

`preflight.rs` checks client/server version compatibility via `/api/v1/info` before running commands; this can be bypassed with `--bypass-version-check`.

For `app:create`, files are collected via `utils/files.rs:collect_files()`, gzip-compressed and base64-encoded. It supports `.scottyignore` (gitignore-style patterns via the `ignore` crate) and auto-excludes `.DS_Store` and `.git/`.

Auth is OAuth device flow plus bearer tokens via env vars or CLI args, implemented under `auth/` (device flow, token storage, caching).
