## 1. Server: remove the setting and response field

- [x] 1.1 Remove `oauth2_proxy_base_url` field and its default from `OAuthSettings` in `scotty-core/src/settings/api_server.rs`
- [x] 1.2 Remove `oauth2_proxy_base_url` from the response `OAuthConfig` in `scotty-core/src/api/server_info.rs`
- [x] 1.3 Remove the field population and the `bind_address`→`localhost` fallback in `scotty/src/api/rest/handlers/info.rs`

## 2. Client: target the user-provided server

- [x] 2.1 Remove `scotty_server_url` from `scottyctl`'s internal `OAuthConfig` in `scottyctl/src/auth/mod.rs`
- [x] 2.2 Remove the `oauth2_proxy_base_url` read and `scotty_server_url` assignment in `scottyctl/src/auth/config.rs`
- [x] 2.3 Build `/oauth/device` and `/oauth/device/token` from `self.user_provided_server_url` in `scottyctl/src/auth/device_flow.rs` (annotated the now-unread `config` field with `#[allow(dead_code)]` to keep its validation flowing)

## 3. Frontend types

- [x] 3.1 Remove `oauth2_proxy_base_url` from the hand-maintained `OAuthConfig` in `frontend/src/types.ts` (ts-generator bin is `scotty-ts-generator`; it emits only `frontend/src/generated/*` and does not cover `OAuthConfig`/`ServerInfo`, so this type is edited by hand)

## 4. Verify

- [x] 4.1 `cargo build` and `cargo test` pass across the workspace (668 passed; `cargo clippy` clean)
- [x] 4.2 `cd frontend && bun run check` passes (0 errors)
- [ ] 4.3 Manual check: `scottyctl --server https://scotty.showcase.factorial.io auth:login` sends device requests to that host, not localhost (confirm via `--debug` / device URL log)
