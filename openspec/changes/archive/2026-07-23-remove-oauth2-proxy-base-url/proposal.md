## Why

`scottyctl auth:login` against a remote server fails: the client hits `http://localhost:21342/oauth/device` instead of the `--server` the user specified. The client trusts a server-reported address (`oauth2_proxy_base_url`) that defaults to the server's `bind_address` (`0.0.0.0:21342` → `localhost:21342`) whenever the deployment did not explicitly set it. That address is redundant — the client already reached the server via `--server`, and the device-flow endpoints live on the server itself.

## What Changes

- Fix `scottyctl` device flow to build `/oauth/device` and `/oauth/device/token` from the user-provided server URL (`--server` / `SCOTTY_SERVER`), which the client already carries as `user_provided_server_url`, instead of the server-reported address.
- **BREAKING** (config surface): Remove the `api.oauth.oauth2_proxy_base_url` server setting. It has no consumer other than the info endpoint being removed below. Leftover config keys are ignored (no `deny_unknown_fields`), so existing deployments do not error.
- **BREAKING** (API surface): Remove the `oauth2_proxy_base_url` field from the `/api/v1/info` `OAuthConfig` response and regenerate the frontend TypeScript types. The frontend and web OAuth flow never consumed it.
- Remove the now-dead `scotty_server_url` field from `scottyctl`'s internal `OAuthConfig` and the code in `info.rs` that derived the localhost fallback from `bind_address`.

Out of scope: the deprecated `api.oauth.frontend_base_url` migration and the `api.oauth.redirect_url` default (`/oauth2/start`) — both are separate oauth2-proxy-era fossils left for a future cleanup.

## Capabilities

### New Capabilities
- `cli-auth-device-flow`: How `scottyctl` resolves which server it authenticates against during the OAuth device flow, and which server-reported OAuth configuration it does and does not depend on.

### Modified Capabilities
<!-- No existing spec captures this behavior; nothing to modify. -->

## Impact

- `scotty-core/src/settings/api_server.rs` — remove `oauth2_proxy_base_url` from `OAuthSettings` (field + default).
- `scotty-core/src/api/server_info.rs` — remove `oauth2_proxy_base_url` from the response `OAuthConfig`.
- `scotty/src/api/rest/handlers/info.rs` — remove the field population and its `bind_address`→localhost fallback.
- `scottyctl/src/auth/mod.rs`, `scottyctl/src/auth/config.rs`, `scottyctl/src/auth/device_flow.rs` — drop `scotty_server_url`; use `user_provided_server_url` for the device endpoints.
- `frontend/src/types.ts` — regenerate via `cargo run --bin ts-generator` (removes the unused field).
- No change to the UI web OAuth flow (`/oauth/authorize`, `/api/oauth/callback`, `/oauth/exchange`), which uses same-origin relative URLs plus `api.base_url`.
