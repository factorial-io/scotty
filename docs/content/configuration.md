# Configuration

Configuration on the server side is done using some toml files inside the
config folder, see below.

The cli does not have any configuration. All it needs to know is the URL of the
server and the api token to authenticate against the server.

## the Cli

Run `scottyctl` with the follwinng options:

```shell
scottyctl --server <SERVER> --access-token <TOKEN>
scottyctl --server https://loclahost:21342 --token my-secret
```

You can also set the environment variables `SCOTTY_SERVER` and
`SCOTTY_ACCESS_TOKEN` to store the server and token for the cli.

To check if the server and access-token works, run the command `app:list`:

```shell
scottyctl --server https://loclahost:21342 --access-token my-secret app:list
```

## the Server

The server has a bunch of configuration files in a folder named `config` on the
same level as the binary. It supports overring specific configuration entries
via env-vars or by entire files. Best practice is to setup your app in
`config/local.yaml` and pass sensitive data via env-vars.

We'll describe in this file all sections of the server configuration:o

### Global settings

```yaml
debug: false
telemetry: None
frontend_directory: ./frontend/build
```
* `debug`: If set to true, the server will log more information. The default is
  false.
* `telemetry`: The telemetry backend to use. The default is `None`. Possible values
  are `None`, `traces` and `metrics`. Please set also the Opentelemetry endpoint
  via the env-var OTEL_where to deliver the traces or metrics. (Use this setting only for debugging)
* frontend_directory: The directory where the frontend is located. The default is
  `./frontend/build`. If you want to use a different frontend, you can set the
  directory here. All files in the directory are served by scotty as static files
  from `/`.

### API settings

```yaml
api:
  bind_address: "0.0.0.0:21342"
  bearer_tokens:
    admin: "placeholder-will-be-overridden" 
    client-a: "placeholder-will-be-overridden"
    deployment: "placeholder-will-be-overridden"
  create_app_max_size: "50M"
  auth_mode: "bearer"  # "dev", "oauth", or "bearer"
  dev_user_email: "dev:system:internal"
  dev_user_name: "Dev User"
  oauth:
    oidc_issuer_url: "https://gitlab.com"
    client_id: "your_client_id"
    client_secret: "your_client_secret"
    redirect_url: "http://localhost:21342/api/oauth/callback"
    frontend_base_url: "http://localhost:21342"
```

* `bind_address`: The address and port the server listens on.
* `bearer_tokens`: **Required for bearer authentication**. Map of logical token identifiers to secure bearer tokens. **Security Note**: Never store actual bearer tokens in configuration files - use placeholder values and override with environment variables (see security best practices below).
* `create_app_max_size`: The maximum size of the uploaded files. The default
  is 50M. As the payload gets base64-encoded, the actual possible size is a
  bit smaller (by ~ 2/3)
* `auth_mode`: Authentication mode. Options are:
  * `"dev"`: Development mode with no authentication (uses fixed dev user)
  * `"oauth"`: Native OAuth authentication with OIDC providers (supports optional bearer token fallback for service accounts)
  * `"bearer"`: Traditional token-based authentication (default)
* `dev_user_email`: Email address for the development user (used when `auth_mode` is "dev")
* `dev_user_name`: Display name for the development user (used when `auth_mode` is "dev")
* `oauth`: OAuth configuration section (used when `auth_mode` is "oauth")
  * `oidc_issuer_url`: OIDC provider URL (e.g., "https://gitlab.com", "https://auth0.com", etc.)
  * `client_id`: OAuth application client ID from your OIDC provider
  * `client_secret`: OAuth application client secret from your OIDC provider
  * `redirect_url`: OAuth callback URL - must match your provider's configuration (backend endpoint)
  * `frontend_base_url`: Base URL of your frontend application for post-authentication redirects (default: "http://localhost:21342")

**Hybrid Authentication:** When `auth_mode` is `oauth`, you can optionally configure `bearer_tokens` to enable service account access alongside OAuth for human users. This allows:
- **Human users** authenticate via OAuth (web UI, CLI device flow)
- **Service accounts** (CI/CD, monitoring, automation) authenticate via bearer tokens
- **Zero OAuth latency** for service accounts (bearer tokens checked first)

See [OAuth Authentication - Hybrid Mode](oauth-authentication.html#hybrid-authentication-oauth-bearer-tokens) for complete documentation on hybrid authentication setup, RBAC configuration, and migration guide.

### Rate Limiting

Scotty includes comprehensive rate limiting to protect API endpoints from abuse and ensure fair resource usage. Rate limiting is configured per-tier with independent limits for different endpoint types.

**Rate limiting is disabled by default** and must be explicitly enabled via configuration.

#### Configuration

Rate limiting is configured in the `api.rate_limiting` section:

```yaml
api:
  rate_limiting:
    enabled: true
    public_auth:
      requests_per_minute: 60
      burst_size: 10
    oauth:
      requests_per_minute: 120
      burst_size: 20
    authenticated:
      requests_per_minute: 300
      burst_size: 50
```

#### Configuration Options

* `enabled`: Global toggle for all rate limiting (default: `false`)
* `public_auth`: Rate limits for login endpoints
  * `requests_per_minute`: Maximum requests allowed per minute
  * `burst_size`: Number of requests allowed in a short burst
* `oauth`: Rate limits for OAuth flow endpoints
  * `requests_per_minute`: Maximum requests allowed per minute
  * `burst_size`: Number of requests allowed in a short burst
* `authenticated`: Rate limits for authenticated API endpoints
  * `requests_per_minute`: Maximum requests allowed per minute
  * `burst_size`: Number of requests allowed per minute

#### Rate Limiting Tiers

Scotty implements three independent rate limiting tiers:

1. **Public Auth** (`public_auth`) - Protects login endpoints from brute force attacks
   * Rate limited by client IP address
   * Applies to: `/api/v1/login`
   * Recommended: 60 requests/minute with burst of 10

2. **OAuth** (`oauth`) - Prevents OAuth flow abuse and session exhaustion
   * Rate limited by client IP address
   * Applies to: `/oauth/device`, `/oauth/authorize`, `/oauth/device/token`, etc.
   * Recommended: 120 requests/minute with burst of 20

3. **Authenticated** (`authenticated`) - Ensures fair resource usage per user
   * Rate limited by bearer token (per-user limits)
   * Applies to: All `/api/v1/authenticated/*` endpoints
   * Recommended: 300 requests/minute with burst of 50

#### Monitoring

Rate limiting metrics are automatically exported to your observability stack:

* `scotty.rate_limit.hits.total` - Total rate limit hits across all tiers
* `scotty.rate_limit.hits.by_tier{tier="..."}` - Hits per tier (authenticated, public_auth, oauth)

These metrics are visualized in the Grafana dashboard under the "Rate Limiting" section.

#### Behavior

When a client exceeds their rate limit:
* HTTP 429 (Too Many Requests) status code is returned
* Response headers include:
  - `Retry-After`: Seconds until the client can retry (RFC 6585)
  - `X-RateLimit-After`: Same information in alternate format
* Metrics are recorded for monitoring and alerting

**Note:** The current implementation does not include additional RFC 6585 headers (`X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`). These may be added in future versions for enhanced client-side rate limit awareness.

#### Deployment Considerations

**Single Instance Deployments**

The current rate limiting implementation uses in-memory token bucket counters. For single-instance deployments, this provides:
* Fast, efficient rate limiting with O(1) lookups
* No external dependencies (Redis, database, etc.)
* Zero latency for rate limit checks

**Multi-Instance Deployments**

For deployments with multiple Scotty instances (horizontal scaling), be aware:

* **Rate limits are per-instance**, not global
  - Each instance maintains its own rate limit counters
  - Example: With 3 instances and 60 req/min limit, effective limit is ~180 req/min

* **Recommended Solutions for Distributed Deployments:**
  1. **External Load Balancer Rate Limiting** (Recommended)
     - Use Nginx, Traefik, or cloud load balancer rate limiting
     - Provides global rate limits across all instances
     - Example: Traefik `RateLimit` middleware, Nginx `limit_req_zone`

  2. **API Gateway**
     - Use AWS API Gateway, Kong, or similar
     - Centralized rate limiting with additional features

  3. **Session Affinity / Sticky Sessions**
     - Route same client IP to same instance
     - Provides consistent rate limiting per client
     - Less effective for distributed attacks

**Future Enhancements**

For distributed rate limiting support, future versions may add:
* Redis-backed rate limiting for shared state
* Configurable rate limit storage backend
* Rate limit synchronization across instances

#### IPv6 Support

**IP-Based Rate Limiting (Public Auth & OAuth Tiers)**

The `SmartIpKeyExtractor` used for IP-based rate limiting supports both IPv4 and IPv6:

* IPv6 addresses are extracted and used as rate limit keys
* Supports standard IPv6 notation (e.g., `2001:db8::1`)
* Handles IPv4-mapped IPv6 addresses (e.g., `::ffff:192.0.2.1`)

**Important IPv6 Considerations:**

1. **Prefix-Based Attacks**
   - IPv6 allows easy generation of many addresses from single /64 prefix
   - Consider implementing prefix-based rate limiting at load balancer level
   - Example: Rate limit entire /64 subnet instead of individual addresses

2. **Proxy Detection**
   - Ensure `X-Forwarded-For` headers are properly handled
   - Configure trusted proxy addresses in your load balancer
   - See [Traefik ForwardedHeaders](https://doc.traefik.io/traefik/routing/entrypoints/#forwarded-headers) or [Nginx real_ip_module](http://nginx.org/en/docs/http/ngx_http_realip_module.html)

3. **Testing**
   - Test rate limiting with both IPv4 and IPv6 clients
   - Verify proxy header forwarding in your deployment

**Example Nginx Configuration for IPv6 Rate Limiting:**

```nginx
# Define rate limit zone supporting both IPv4 and IPv6
limit_req_zone $binary_remote_addr zone=scotty_limit:10m rate=60r/m;

server {
    listen 80;
    listen [::]:80;  # IPv6 support

    location /api/v1/login {
        limit_req zone=scotty_limit burst=10;
        proxy_pass http://scotty_backend;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

### Authorization settings

Scotty includes an optional scope-based authorization system for controlling access to applications and operations. See the [Authorization System](authorization.html) documentation for complete details.

**Authorization is entirely optional** - if no configuration is provided, Scotty operates with the existing all-or-nothing access model.

#### Configuration Files

Authorization requires two configuration files in the `config/casbin/` directory:

```
config/
├── casbin/
│   ├── model.conf       # Casbin RBAC model (auto-generated)
│   └── policy.yaml      # Groups, roles, and assignments
└── default.yaml         # Main configuration
```

#### Example Authorization Configuration

Create `config/casbin/policy.yaml` with your access control setup:

```yaml
# Scope definitions - organize apps by purpose
scopes:
  frontend:
    description: "Frontend applications"
    created_at: "2023-12-01T00:00:00Z"
  backend: 
    description: "Backend services"
    created_at: "2023-12-01T00:00:00Z"
  production:
    description: "Production environment"
    created_at: "2023-12-01T00:00:00Z"

# Role definitions with permissions
roles:
  admin:
    description: "Full administrative access"
    permissions: ["*"]  # Wildcard for all permissions
    created_at: "2023-12-01T00:00:00Z"
  developer:
    description: "Development access"
    permissions: ["view", "manage", "shell", "logs", "create"]
    created_at: "2023-12-01T00:00:00Z"
  operator:
    description: "Operations access without shell"
    permissions: ["view", "manage", "logs"]
    created_at: "2023-12-01T00:00:00Z"

# User/token assignments to roles within scopes
# Assignment key format depends on authentication mode:
#
# OAuth Mode (auth_mode: "oauth"):
#   - Use email addresses directly: "user@example.com"
#   - Email is extracted from OIDC token claims during authentication
#
# Bearer Mode (auth_mode: "bearer"):
#   - Use identifier prefix: "identifier:token_name"
#   - Maps to configured bearer_tokens (api.bearer_tokens.token_name)
#
# Dev Mode (auth_mode: "dev"):
#   - Uses fixed dev user from api.dev_user_* configuration
#   - Authorization assignments are not applicable
assignments:
  # OAuth user assignments (when auth_mode: "oauth")
  "alice@example.com":
    - role: "admin"
      scopes: ["*"]  # Global access to all scopes
  "stephan@factorial.io":
    - role: "admin"
      scopes: ["*"]  # Global admin access
  "bob@example.com":
    - role: "developer"
      scopes: ["frontend", "backend"]

  # Bearer token assignments (when auth_mode: "bearer")
  "identifier:deployment":  # Maps to bearer_tokens.deployment
    - role: "developer"
      scopes: ["staging"]
  "identifier:admin":       # Maps to bearer_tokens.admin
    - role: "admin"
      scopes: ["*"]

# App scope mappings (managed automatically from .scotty.yml)
apps:
  "my-frontend-app": ["frontend"]
  "my-backend-api": ["backend"]
```

#### App Scope Assignment

Apps declare scope membership in their `.scotty.yml` configuration:

```yaml
# Apps can belong to multiple scopes
scopes:
  - "frontend"
  - "staging"

public_services:
  - service: "web"
    port: 3000
```

#### Available Permissions

- `view` - See app status and information
- `manage` - Start, stop, restart applications
- `logs` - View application logs
- `shell` - Execute shell commands in containers
- `create` - Create new apps in scope
- `destroy` - Delete apps from scope

#### Bearer Token Integration

Bearer tokens are configured using logical identifiers that map to secure tokens:

1. **Configuration**: Define secure tokens in `api.bearer_tokens` section
2. **Authorization**: Reference identifiers in policy assignments as `identifier:name`
3. **Environment Override**: Use `SCOTTY__API__BEARER_TOKENS__NAME=secure_token`

Example CLI usage with authorized token:
```bash
export SCOTTY_ACCESS_TOKEN="secure-admin-token-abc123"
scottyctl app:list  # Shows only apps user has 'view' permission for
```

**Important**: The `api.access_token` configuration is **no longer supported**. Use `api.bearer_tokens` instead.

#### Bearer Token Security Best Practices

🔒 **NEVER store actual bearer tokens in configuration files!** Follow these security guidelines:

##### 1. Use Environment Variables for Actual Tokens

Store only placeholder values in configuration files and override with secure environment variables:

```bash
# Production deployment - set actual secure tokens via environment variables
export SCOTTY__API__BEARER_TOKENS__ADMIN="$(openssl rand -base64 32)"
export SCOTTY__API__BEARER_TOKENS__DEPLOYMENT="$(openssl rand -base64 32)"
export SCOTTY__API__BEARER_TOKENS__CLIENT_A="$(openssl rand -base64 32)"

# Start Scotty server
./scotty
```

##### 2. Generate Strong Tokens

Use cryptographically secure random tokens:

```bash
# Generate 32-byte base64-encoded tokens (recommended)
openssl rand -base64 32

# Or use system UUID (less entropy but still secure)
uuidgen
```

##### 3. Configuration File Security

In your `config/local.yaml` or `config/default.yaml`:

```yaml
api:
  bearer_tokens:
    admin: "OVERRIDE_VIA_ENV_VAR"           # Will be overridden by SCOTTY__API__BEARER_TOKENS__ADMIN
    deployment: "OVERRIDE_VIA_ENV_VAR"      # Will be overridden by SCOTTY__API__BEARER_TOKENS__DEPLOYMENT
    monitoring: "OVERRIDE_VIA_ENV_VAR"      # Will be overridden by SCOTTY__API__BEARER_TOKENS__MONITORING
```

##### 4. Token Rotation

Regularly rotate bearer tokens:

1. Generate new secure tokens
2. Update environment variables
3. Restart Scotty server
4. Update CLI configurations and automation tools

##### 5. Access Control

- **Principle of Least Privilege**: Create different tokens with different permissions via authorization system
- **Scope Limitation**: Use authorization scopes to limit what each token can access
- **Audit Regularly**: Review token assignments and remove unused tokens

Example secure deployment setup:

```bash
#!/bin/bash
# secure-deploy.sh - Production deployment script

# Generate secure tokens if they don't exist
export SCOTTY__API__BEARER_TOKENS__ADMIN="${ADMIN_TOKEN:-$(openssl rand -base64 32)}"
export SCOTTY__API__BEARER_TOKENS__DEPLOY="${DEPLOY_TOKEN:-$(openssl rand -base64 32)}"

# Set restrictive file permissions
chmod 700 config/
chmod 600 config/*.yaml

# Start server with secure token environment
exec ./scotty
```

###  Scheduler settings

scotty is running some tasks in the background on a regular level. Here you can
fine tune when certain tasks are executed:

```yaml
scheduler:
  running_app_check: "15s"
  ttl_check: "10m"
  task_cleanup: "3m"
```

* `running_app_check` how often should the app-folder be traversed and the
  running apps be checked. The default is 15s.
* `ttl_check` how often should the ttl of the apps be checked. The default is
  10m. This setting describes the frequency of the ttl-check. The actual ttl is
  configured on a per app basis.
* `task_cleanup` how often should the task-queue be cleaned up. The default is
  3m. Everytime a cli or UI is issuing a command, a new task gets created and
  put into the queue. The task encapsulates the output of the command and
  other useful information. The higher the setting the longer you can inspect
  the output of commands in the UI

### App settings

```
apps:
  domain_suffix: "ddev.site"
  root_folder: "./apps" # Path to the folder where the apps are stored
```

* `domain_suffix` The suffix for auto-generated domains. Set this to the domain
  you want to use for your apps. App-domains are constructed by the service- and
  the app-name, when a custom domain is not provided.
* `root_folder` The folder where the apps are stored. The default is `./apps`.
  If you run scotty in a docker container, and mount the apps-folder into the
  container, make sure that both paths are the same. Otherwise docker-compose
  can't run the apps, as there is a mismatch between the local path and the
  path on the host, where the docker daemon is running.

### Docker settings

Scotty uses docker to inspect running docker apps. For this to work it needs to
communicate with the docker daemon

```yaml
docker:
  connection: local # local, socket or http, see bollard docs
  registries:
    example_registry:
      registry: https://registry.example.com
      username: "registry"
      password: "registry"
    example_2_registry:
      ...
```

* `connection` The connection to the docker daemon. The default is `local`. Other
  possible values are `socket` and `http`. See the bollard documentation for
  more information.
* `registries` A list of registries to pull images from. The key is the name of
  the registry, the value is a map with the keys `registry`, `username` and
  `password`. The password is stored in plain text. If you want to use a
  registry with a token, you need to provide the token as password.

  The key for the registry is used when creating a new app, so scotty knows
  where to pull the image from, see the docs for `app:create`

  If you do not want to store the password in plain text, you can provide the
  password as an environment variable. Check the override section for more
  information.

### Loadbalancer settings

Scotty can work with different loadbalancers, currently with Traefik (preferred)
and Haproxy-config (deprecated).

```yaml
load_balancer_type: Traefik #HaproxyConfig or Traefik
traefik:
  network: "proxy"
  use_tls: true
  certresolver: "myresolver"
haproxy:
  use_tls: true
```

* `load_balancer_type` The loadbalancer to use. Use `Traefik` or `HaproxyConfig`

#### Traefik

* `network` The network to use for the communication between scotty and traefik.
  The default is `proxy`. If you use a different network, make sure to create
  the network before starting scotty.
  Scotty will also add the network to all public services of your app when you
  create or adopt an app, so traefik can access the public services of the app.
* `use_tls` If set to true, scotty will create the necessary labels for traefik
  to use tls. The default is true.
* `certresolver` The certresolver to use for the tls-certificate. The
  certresolver must be configured in traefik. The default is `myresolver` shown
  also in the example `docker-compose.yml` from the [installation-documentation](installation.md)

#### Haproxy-config

* `use_tls` If set to true, scotty will create the necessary environment variables
  for haproxy-config to use tls.


### Blueprints

Blueprints are a way to run certain tasks after specific events happened, like
`app:create`, `app:run` or `app:destroy`. They are stored in `config/blueprints`
and can be adopted by apps. They are stored as separate files in the
`config/blueprints`-folder. Scotty is using the key of the blueprint to associate
the blueprint with an app.

Here's an example blueprint:

```yaml
apps:
  blueprints:
    drupal-lagoon:
      name: "Drupal using lagoon base images"
      description: "A simple Drupal application using lagoon base images (cli, php, nginx)"
      required_services:
        - cli
        - php
        - nginx
      public_services:
        nginx: 8080
      actions:
        post_create:
          cli:
            - drush deploy
        post_rebuild:
          cli:
            - drush deploy
        post_run:
          cli:
            - drush uli
```

* `name` The name of the blueprint
* `description` A short description of the blueprint
* `required_services` A list of services that are required for the blueprint to
  work. If one of the services is missing, scotty will throw an error when an
  app tries to adopt the blueprint.
* `public_services` A list of services that should be exposed to the public. The
  key is the service name, the value is the port to expose.
* `actions` A list of action-hooks and their corresponding commands. The
  following hooks are available:
  * `post_create` Run after the app was created
  * `post_rebuild` Run after the app was rebuilt
  * `post_run` Run after the app was started
  * `post_destroy` Run after the app was destroyed
  The key is the service name, the value is a list of commands to run on that
  service.

#### Environment Variables in Blueprint Actions

When blueprint action scripts are executed, Scotty automatically injects
additional environment variables that can be used in your commands:

* `SCOTTY__APP_NAME` - Contains the name of the app
* `SCOTTY__PUBLIC_URL__<SERVICE_NAME>` - Contains the public URL for each
  service that has a public URL configured. The service name is sanitized
  to be a valid environment variable name (e.g., `my-service` becomes
  `SCOTTY__PUBLIC_URL__MY_SERVICE`)

These variables are available in addition to any environment variables you've
configured for your app via the `--env` option or in the `.scotty.yml` file.

**Example usage:**

```yaml
apps:
  blueprints:
    drupal-lagoon:
      name: "Drupal using lagoon base images"
      description: "A simple Drupal application using lagoon base images"
      required_services:
        - cli
      public_services:
        nginx: 8080
      actions:
        post_run:
          cli:
            - echo "App name is $SCOTTY__APP_NAME"
            - echo "Public URL is $SCOTTY__PUBLIC_URL__NGINX"
            - drush uli --uri="$SCOTTY__PUBLIC_URL__NGINX"
```

If you create a new app via `app:create` or the REST-API, you can provide the
blueprint to associate with your app.

### 1Password settings

Scotty can integrate with 1Password connect and resolve secrets from
one or more connect instances when needed. For that to work, scotty can parse
a special uri-scheme for envuronment variables, before running any action on an
app. Here's the URI-scheme:

```
op://<connect-instance>/<vault-uuid>/<item-uuid>/<field>
```

Scotty needs a JWT token to authenticate against a connect instance. The JWT
token is stored in the configuration file:

```yaml
onepassword:
  connect-instance-a:
    jwt: todo
    server: https://connect-a.example.com
  connect-instance-b:
    jwt: todo
    server: https://connect-b.example.com
```

Then you can inject the actual JWT-tokens via the environment variables:

```shell
export SCOTTY__ONEPASSWORD__CONNECT_INSTANCE_A__JWT=todo
export SCOTTY__ONEPASSWORD__CONNECT_INSTANCE_B__JWT=todo
```

Then, for your app, you can set a environment variable like this:

```shell
scottyctl app:create test ... --env "DATABASE_PASSWORD=op://connect-instance-a/vault-uuid/item-uuid/password"
```

Scotty will resolve the secret from the connect instance and inject the value
when running an action on the app. Please note, that it won't resolve secrets
from environment variables inside docker-compose.yml files.

### Notification settings

Scotty supports issuing notifications via multiple channels. THese notifications
are sent on all actions that are run on an app. The following channels are
supported and need to be configured:

#### Mattermost channels

Scotty can send a notification after an action was successfully run on an app to
a mattermost channel. Here's an example configuration:

```yaml
notifications:
  mattermost-example:
    type: mattermost
    host: "https://mattermost.example.com"
    hook_id: "some-hook-id"
```

The hook_id is the id of the incoming webhook in mattermost. You can create a
new incoming webhook in the mattermost settings.

#### Gitlab merge requests

```yaml
notifications:
  gitlab-example:
    type: gitlab
    host: "https://gitlab.example.com
    token: "some-token"
```

Scotty needs a personal access token to authenticate against the gitlab instance.
The token must have the `api`-scope. You can create a new personal access token
in the gitlab settings.

#### Webhooks

```yaml
notifications:
  webhook-example:
    type: webhook
    method: "POST"
    url: "https://webhook.example.com"
```

Scotty will send a POST-request to the url with the payload of the notification.
Method can be `POST` or `GET`.

### How to override the configuration

The default configuration is stored in `config/default.yaml`. You can override
all or parts of the documentation by creating a file `config/local.yaml` and
replace the values you want to override.

As an alternative you can override the configuration by setting environment
variables, this is especiall useful for sensitive data like passwords.

The environment variables must be prefixed with `SCOTTY__` and the keys must be
concatenated with *double underscores*. For example to override bearer tokens
you can set environment variables like `SCOTTY__API__BEARER_TOKENS__ADMIN`.

Rule of thumb is: If you want to override a key, replace the dots with double
underscores and prefix the key with `SCOTTY__`.

#### Using .env files for configuration

**Recommended for local development**: Scotty automatically loads `.env` and `.env.local` files on startup, making it easy to configure the server without managing environment variables manually.

**Configuration precedence** (highest to lowest priority):

1. **Environment variables** - Always take precedence over .env files
2. **`.env.local`** - Local overrides, not committed to version control
3. **`.env`** - Shared defaults, can be committed to version control

**Example `.env` file:**

```bash
# Development mode configuration
SCOTTY__API__AUTH_MODE=dev

# Docker registry credentials
SCOTTY__DOCKER__REGISTRIES__MYREGISTRY__PASSWORD=secret123

# Bearer tokens (use strong tokens in production!)
SCOTTY__API__BEARER_TOKENS__ADMIN=my-secure-admin-token
```

**Example `.env.local` file (overrides .env):**

```bash
# Override for local development only
SCOTTY__API__AUTH_MODE=oauth
SCOTTY__API__OAUTH__CLIENT_ID=my-local-client-id
```

**Best practices:**

- ✅ Use `.env` for shared development defaults (can be committed)
- ✅ Use `.env.local` for personal overrides (add to `.gitignore`)
- ✅ Generate strong tokens for production (see security best practices above)
- ❌ Never commit secrets to version control
- ❌ Don't rely on .env files in production (use proper environment variables)

**Note:** Both `.env` and `.env.local` files are optional - if they don't exist, the server will start normally using configuration files and environment variables.

#### Example

| name of value in the config file                  | environment variable                                     |
|---------------------------------------------------|----------------------------------------------------------|
| `debug`                                           | `SCOTTY__DEBUG`                                          |
| `api.bearer_tokens.admin`                         | `SCOTTY__API__BEARER_TOKENS__ADMIN`                      |
| `api.bearer_tokens.deployment`                    | `SCOTTY__API__BEARER_TOKENS__DEPLOYMENT`                 |
| `api.bind_address`                                | `SCOTTY__API__BIND_ADDRESS`                              |
| `api.auth_mode`                                   | `SCOTTY__API__AUTH_MODE`                                 |
| `api.dev_user_email`                              | `SCOTTY__API__DEV_USER_EMAIL`                            |
| `api.dev_user_name`                               | `SCOTTY__API__DEV_USER_NAME`                             |
| `api.oauth.oidc_issuer_url`                       | `SCOTTY__API__OAUTH__OIDC_ISSUER_URL`                    |
| `api.oauth.client_id`                             | `SCOTTY__API__OAUTH__CLIENT_ID`                          |
| `api.oauth.client_secret`                         | `SCOTTY__API__OAUTH__CLIENT_SECRET`                      |
| `api.oauth.redirect_url`                          | `SCOTTY__API__OAUTH__REDIRECT_URL`                       |
| `api.oauth.frontend_base_url`                     | `SCOTTY__API__OAUTH__FRONTEND_BASE_URL`                  |
| `docker.registries.example_registry.password`     | `SCOTTY__DOCKER__REGISTRIES__EXAMPLE_REGISTRY__PASSWORD` |
| `apps.domain_suffix`                              | `SCOTTY__APPS__DOMAIN_SUFFIX`                            |
| `load_balancer_type`                              | `SCOTTY__LOAD_BALANCER_TYPE`                             |
| `traefik.network`                                 | `SCOTTY__TRAEFIK__NETWORK`                               |

Scotty will print out the resolved configuration on startup, so you can check
for any errors.
