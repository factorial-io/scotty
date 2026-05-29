---
# scotty-6oix
title: Fix rustls CryptoProvider panic in scottyctl/scotty TLS startup
status: completed
type: bug
priority: high
created_at: 2026-05-29T08:45:55Z
updated_at: 2026-05-29T08:49:18Z
---

scottyctl panics at runtime with 'Could not automatically determine the process-level CryptoProvider from Rustls crate features' when making HTTPS requests (e.g. app:create against https endpoint). Deployed binary resolves rustls 0.23.40 (un --locked Docker build) where the ring/aws-lc-rs provider can't be auto-selected. Fix: explicitly install a rustls CryptoProvider at process start in main(), plus harden the Docker build.

## Summary of Changes

Root cause: the Docker image built scottyctl/scotty without `--locked`, so dependency resolution drifted from the committed Cargo.lock (rustls 0.23.20) to rustls 0.23.40. At that version rustls could not auto-select a crypto provider (ring vs aws-lc-rs ambiguity), panicking on the first TLS handshake — e.g. `app:create` against an https endpoint.

Fix:
- Added `scotty_core::http::ensure_crypto_provider()` (new `scotty-core/src/http/tls.rs`) which installs the rustls `ring` provider once via `Once`. Version-independent and idempotent.
- Called it at the top of `scottyctl` main() and `scotty` server main(), before any TLS use.
- Added `rustls` (ring, std) as a workspace dep and to scotty-core.
- Hardened Dockerfile with `--locked` on all cargo chef cook / build / run steps to stop future resolution drift.

Verified: `cargo build` (358 crates) clean, clippy clean, 70 scottyctl tests pass.
