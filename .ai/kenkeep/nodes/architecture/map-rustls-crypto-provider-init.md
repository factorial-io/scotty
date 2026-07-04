---
type: map
title: rustls CryptoProvider is installed explicitly at process start
description: >-
  scotty and scottyctl both call scotty_core::http::ensure_crypto_provider()
  before any TLS use, and Docker builds pin dependency versions with --locked.
tags:
  - tls
  - rustls
  - startup
  - docker-build
kk_schema_version: 3
kk_id: map-rustls-crypto-provider-init
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Both `scotty` and `scottyctl` main() functions call `scotty_core::http::ensure_crypto_provider()` (in `scotty-core/src/http/tls.rs`) once via `Once` at startup, before any TLS handshake. This exists because rustls cannot always auto-select a crypto provider (ring vs aws-lc-rs) when multiple provider features are reachable in the dependency graph, which otherwise panics on first HTTPS use.

The Docker image build also always builds with `--locked` (cargo chef cook, build, run) so the resolved dependency graph, including rustls's exact version, can't silently drift from the committed Cargo.lock between local dev and the image.
