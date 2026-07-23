## Context

Scotty has several "base URL"-ish settings that are easy to conflate. Investigation (`/opsx:explore`) established what each actually drives:

| Setting | Drives | Keep? |
|---|---|---|
| `--server` / `SCOTTY_SERVER` (scottyctl) | How this client reaches Scotty; used for every API call incl. `/api/v1/info` | Yes — canonical |
| `api.base_url` (server, via `public_base_url()`) | Scotty's public origin: landing-page redirects, OAuth web-callback fallback, startup warnings | Yes |
| `api.oauth.frontend_base_url` | Deprecated alias for `api.base_url` | Out of scope |
| `api.oauth.oauth2_proxy_base_url` | Only feeds the `/api/v1/info` response, which the device-flow client reads as its target address; defaults to `bind_address` → `localhost:21342` | Remove |

Key facts that make the removal safe:
- The device-flow **browser URL** (`verification_uri`) comes from the OIDC provider (`scotty/src/oauth/device_flow.rs:36`, `details.verification_uri()`), not from Scotty. Scotty's public URL is irrelevant to the browser step.
- During `auth:login` the client only needs Scotty's address for two of Scotty's own endpoints (`/oauth/device`, `/oauth/device/token`). It already reached `/api/v1/info` via `--server`, so `--server` reaches those too.
- The UI web OAuth flow never reads `oauth2_proxy_base_url`: the frontend uses same-origin relative URLs; the IdP callback uses `api.oauth.redirect_url`; the final redirect fallback uses `api.base_url`.
- `scottyctl` already carries the user-provided URL as `DeviceFlowClient.user_provided_server_url`, currently used only to stamp the stored-token record — not for the device requests.

## Goals / Non-Goals

**Goals:**
- `scottyctl auth:login` works against any `--server`, including remote deployments that never set `oauth2_proxy_base_url`.
- Delete the redundant setting, response field, and localhost-fallback code so the failure mode cannot recur.
- Keep the info-response contract honest: no server-self-address the client must trust.

**Non-Goals:**
- Migrating or removing `api.oauth.frontend_base_url` (already deprecated on its own path).
- Changing `api.oauth.redirect_url` (`/oauth2/start`) — a separate oauth2-proxy fossil.
- Any change to the UI web OAuth flow.
- Re-litigating whether `client_id` / `oidc_issuer_url` should stay in the info response (see Open Questions).

## Decisions

**Decision: Client uses `user_provided_server_url` for device endpoints.**
`DeviceFlowClient::start_device_flow` and `try_get_token` build their URLs from `self.user_provided_server_url` instead of `self.config.scotty_server_url`. Rationale: the value is already present and is the address the user explicitly chose; the server's self-report is at best redundant and at worst wrong.
- Alternative considered: keep the field but fix the server's fallback to emit its public URL. Rejected — it keeps a setting deployments must remember to configure, and the client already has the correct value locally.

**Decision: Remove `scotty_server_url` from `scottyctl`'s internal `OAuthConfig`.**
Once the device endpoints use `user_provided_server_url`, the field is dead. Removing it also removes the `InvalidResponse` gate in `config.rs` that required the server to report an address.

**Decision: Remove the setting and response field entirely rather than deprecate.**
Nothing consumes the setting except the code being deleted, and the response field is unused by the frontend (present only in generated `types.ts`). A deprecation shim would add code for zero consumers. Old config keys are silently ignored (no `deny_unknown_fields` on `OAuthSettings`), so removal is backward-compatible for server startup.

## Risks / Trade-offs

- **API response shape changes for any external consumer of `/api/v1/info`** → The only known consumer is `scottyctl` (updated here) and the frontend (regenerated, doesn't use it). ts-rs regeneration keeps the frontend type in sync. External scripts reading the field would see it disappear — acceptable given it only ever echoed a localhost default.
- **A deployment relied on `oauth2_proxy_base_url` pointing the CLI somewhere other than `--server`** (e.g. CLI talks to Scotty at one address, device flow at another) → No evidence this is a real topology; the device endpoints are the same Scotty process. If such a split ever existed it was already broken by the localhost default. Accepted.
- **Version skew: new `scottyctl` against an old server** → New client ignores the response field and uses `--server`; works. **Old client against a new server** → field is gone, old client's `config.rs` hits `InvalidResponse` and login fails. Mitigation: this is a client bugfix; users hitting the remote-login bug are already upgrading the client. Note in release notes.

## Migration Plan

1. Update server (remove setting + response field + fallback), regenerate TS types.
2. Update client (use `user_provided_server_url`, drop `scotty_server_url`).
3. Release together via normal release-please flow. Conventional commit `fix:` for the client behavior; the config/response removal is a `fix`/`feat` with a BREAKING-change note in the body for the info-response field.
4. Rollback: revert the change set; no data migration involved.

## Open Questions

- `client_id` and `oidc_issuer_url` remain in the info-response `OAuthConfig` and are presence-validated by the client but their values are unused (native device flow means Scotty talks to the IdP). Leave as-is for this change, or demote to informational / remove the client-side gate? Deferred — not required to fix the bug.
