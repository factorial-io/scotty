## ADDED Requirements

### Requirement: Device flow targets the user-specified server

`scottyctl` SHALL send OAuth device-flow requests (`/oauth/device` and `/oauth/device/token`) to the server URL provided by the user via `--server` or the `SCOTTY_SERVER` environment variable. It SHALL NOT derive the device-flow target from any address reported back by the server in its `/api/v1/info` response.

#### Scenario: Login against a remote server

- **WHEN** a user runs `scottyctl --server https://scotty.example.com auth:login`
- **THEN** the device-flow requests are sent to `https://scotty.example.com/oauth/device` and `https://scotty.example.com/oauth/device/token`
- **AND** no request is sent to `localhost` or the server's internal bind address

#### Scenario: Login against the default local server

- **WHEN** a user runs `scottyctl auth:login` with no `--server` and no `SCOTTY_SERVER` set
- **THEN** the device-flow requests are sent to the default server URL (`http://localhost:21342`)

### Requirement: Server does not advertise its own address for the device flow

The `/api/v1/info` `OAuthConfig` response SHALL NOT include a Scotty server address field, and the server SHALL NOT expose an `oauth2_proxy_base_url` configuration setting for this purpose. The device-flow endpoints are served by Scotty itself, and the client already knows Scotty's address from the connection it used to reach `/api/v1/info`.

#### Scenario: Info response omits the server address

- **WHEN** a client requests `/api/v1/info` from a server configured for OAuth
- **THEN** the returned `oauth_config` contains no `oauth2_proxy_base_url` field

#### Scenario: Legacy config key is ignored, not rejected

- **WHEN** a deployment still sets `api.oauth.oauth2_proxy_base_url` in its configuration
- **THEN** the server starts normally and ignores the unknown key

## REMOVED Requirements

### Requirement: Server-reported OAuth proxy base URL

**Reason**: The `oauth2_proxy_base_url` field is a fossil from an oauth2-proxy sidecar deployment model that native OAuth replaced. Its only consumer was the device-flow client, which now uses the user-provided `--server` URL. When unset it defaulted to the server's bind address, resolving to `localhost:21342` and breaking `auth:login` against remote servers.

**Migration**: Remove `api.oauth.oauth2_proxy_base_url` from server configuration (it is ignored if left in place). Clients on the updated `scottyctl` require no configuration change — the device flow follows `--server` / `SCOTTY_SERVER`.
