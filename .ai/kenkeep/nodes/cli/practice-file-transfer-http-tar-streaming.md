---
type: practice
title: 'File transfers stream over HTTP chunked tar, not WebSocket'
description: >-
  Container file transfer uses HTTP GET/PUT with application/x-tar bodies and
  bounded, streaming I/O on both ends.
tags:
  - file-transfer
  - docker
  - streaming
  - http
kk_schema_version: 3
kk_id: practice-file-transfer-http-tar-streaming
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Scotty moves files between a workstation and a container over HTTP GET/PUT endpoints carrying application/x-tar bodies with chunked transfer encoding, rather than over WebSocket. Docker's own copy API already speaks tar, so reusing that format avoids inventing a custom binary framing protocol; WebSocket is reserved for interactive bidirectional sessions (shell, logs), not one-shot bulk transfers.

Server and client both stream rather than buffer the whole transfer: the server pipes the Docker-client tar stream directly to/from the HTTP body, and the CLI drives the synchronous tar crate from a blocking task connected via channels, so a single helper absorbs the sync-in-async impedance rather than spreading it through the codebase.

Uploaded tar entries are restricted to relative names — the client refuses to pack entries with absolute paths or parent-directory traversal — since the archive is handed to Docker's extraction and the client does not want to construct a hostile archive even though Docker is trusted to sandbox extraction on its side.
