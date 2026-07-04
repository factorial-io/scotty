---
type: practice
title: 'app:cp path-spec parsing and pipe-mode semantics'
description: >-
  app:cp resolves remote/local/stdio arguments docker-cp-style, resolves omitted
  service names from the blueprint, and treats pipe mode as single-file tar with
  lossy metadata.
tags:
  - file-transfer
  - cli
  - scottyctl
kk_schema_version: 3
kk_id: practice-app-cp-path-spec-and-pipe-mode
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
scottyctl app:cp parses each of its two arguments into a local path, the literal "-" (stdin/stdout), or a remote spec app[:service]:path, using the same heuristic docker cp uses to distinguish a remote spec from a Windows drive letter or an existing local path. Exactly one side of the command must resolve to a remote spec.

When the service segment is omitted, scottyctl resolves it client-side from the app's blueprint: if exactly one service is marked public: true, that one is used; otherwise the command fails with an explicit list of candidate service names rather than guessing.

In pipe mode (either side is "-"), the wire format is still a single-file tar archive: the entry name on upload derives from the basename of the remote destination path, and a download aborts if the source path expands to more than one regular-file entry, since pipe mode is inherently single-file. Metadata carried through pipe mode is lossy — mode is fixed at 0644, mtime is set to the current time, and ownership is dropped — whereas file-to-file transfers preserve mode/owner/mtime via the tar entry.
