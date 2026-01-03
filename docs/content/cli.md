# The command line interface

The CLI provides a thin wrapper to access the REST API of Scotty. It is
written in Rust and provides a simple interface to list, create, update and
destroy apps, as well as manage authorization (scopes, roles, assignments).

You can get help by running `scottyctl --help` and `scottyctl --help <command>`.

## Authentication

Scotty supports two authentication methods:

### OAuth Authentication (Recommended)

Use the device flow to authenticate interactively:

```shell
scottyctl --server https://scotty.example.com auth:login
```

This command will:
1. Open your browser to authenticate with the OIDC provider
2. Store the OAuth token securely for future commands
3. Automatically refresh tokens when they expire

**Managing OAuth sessions:**

```shell
# Check authentication status
scottyctl auth:status

# Refresh the token
scottyctl auth:refresh

# Logout and clear stored credentials
scottyctl auth:logout
```

### Bearer Token Authentication

For service accounts, CI/CD, or when OAuth is not available, use bearer tokens:

```shell
# Via command line argument
scottyctl --server https://scotty.example.com --access-token <TOKEN> app:list

# Via environment variable (recommended)
export SCOTTY_SERVER=https://scotty.example.com
export SCOTTY_ACCESS_TOKEN=<TOKEN>
scottyctl app:list
```

**Note:** For the rest of this documentation, command examples use `--server` and `--access-token` for clarity, but you can always use OAuth via `auth:login` or environment variables instead.

## List all apps

```shell
scottyctl --server <SERVER> --access-token <TOKEN> app:list
```

Example output:
![Example output of app:list](assets/cli/app-list.png)

The table contains all apps with their status, uptime and URLs. The URLs are the
public URLs of the apps. The status can be one of the following:

* Running: The app is running
* Stopped: The app is stopped
* Unsupported: The app is not supported by the server

## Get info about an app

```shell
scottyctl --server <SERVER> --access-token <TOKEN> app:info <APP>
```

Example output:
![Example output of app:info](assets/cli/app-info.png)

The command lists all services of a specific app and their status. The output
also contains the enabled notification services for that app.

## View logs from an app service

```shell
scottyctl --server <SERVER> --access-token <TOKEN> app:logs <APP> <SERVICE>
```

This command displays logs from a specific service within an app. By default, it shows all available logs and exits.

### Options

* `-f, --follow`: Follow log output in real-time (like `tail -f`)
* `-n, --lines <LINES>`: Show only the last N lines
* `--since <SINCE>`: Show logs since a timestamp (e.g., "2h", "30m", "2023-01-01T10:00:00Z")
* `--until <UNTIL>`: Show logs until a timestamp (e.g., "1h", "2023-01-01T11:00:00Z")
* `-t, --timestamps`: Include timestamps in the log output

### Examples

View all logs:
```shell
scottyctl app:logs my-app web
```

Follow logs in real-time:
```shell
scottyctl app:logs my-app web --follow
```

Show last 100 lines with timestamps:
```shell
scottyctl app:logs my-app web --lines 100 --timestamps
```

Show logs from the last 2 hours:
```shell
scottyctl app:logs my-app web --since 2h --follow
```

## Open an interactive shell in an app service

```shell
scottyctl --server <SERVER> --access-token <TOKEN> app:shell <APP> <SERVICE>
```

This command opens an interactive shell session inside a running container. This is useful for debugging, inspecting files, or running commands inside the container environment.

**Note:** The shell command requires the `shell` permission in the authorization system.

### Options

* `-c, --command <COMMAND>`: Execute a single command instead of opening an interactive shell. The command will run and scottyctl will exit with the same exit code as the command.
* `--shell <SHELL>`: Specify which shell to use (default: `/bin/bash`)

### Examples

Open an interactive shell:
```shell
scottyctl app:shell my-app web
```

Execute a single command:
```shell
scottyctl app:shell my-app web --command "ls -la /app"
```

Use a different shell:
```shell
scottyctl app:shell my-app web --shell /bin/sh
```

Change to a specific directory and run commands:
```shell
scottyctl app:shell my-app web --command "cd /var/www && pwd && ls -la"
```

Run a script and capture its exit code:
```shell
scottyctl app:shell my-app web --command "/app/scripts/healthcheck.sh"
echo "Exit code: $?"
```

## Start/run an app

```shell
scottyctl --server <SERVER> --access-token <TOKEN> app:start <APP>
scottyctl --server <SERVER> --access-token <TOKEN> app:run <APP>
```

The command will start an app and print the output of the start process. After
the command succeeds, it will print the app info.

## Stop an app

```shell
scottyctl --server <SERVER> --access-token <TOKEN> app:stop <APP>
```

The command will stop an app and print the output of the stop process. After
the command succeeds, it will print the app info.

## Rebuild an app

```shell
scottyctl --server <SERVER> --access-token <TOKEN> app:rebuild <APP>
```

The command will rebuild an app and print the output of the rebuild process.
Part of the rebuild process is rewriting the proxy configuration, pulling new
images for the app and rebuilding local images if necessary. The app itself will
also be powered off and on again.

## Purge an app

```shell
scottyctl --server <SERVER> --access-token <TOKEN> app:purge <APP>
```
The command will purge all temporary data of an app, especially logs,
temporary docker containers and other ephemeral data. It will not delete any
persistent data like volumes or databases. If the app was running, it will be
stopped by this command.

## Create an app

```shell
scottyctl --server <SERVER> --access-token <TOKEN> app:create <APP> --folder <FOLDER> \
  --service <SERVICE:PORT> [--service <SERVICE:PORT> ...] \
  [--app-blueprint <BLUEPRINT>] [--ttl <LIFETIME>] \
  [--basic-auth <USERNAME:PASSWORD>] [--allow-robots] \
  [--destroy-on-ttl] \
  [--custom-domain <DOMAIN:SERVICE>] [--custom-domain <DOMAIN:SERVICE> ...] \
  [--env <KEY=VALUE>] [--env <KEY=VALUE> ...] \
  [--env-file <FILE>] \
  [--registry <REGISTRY>] \
  [--middleware <MIDDLEWARE>] [--middleware <MIDDLEWARE> ...]
```

This command will create a new app on the server. The `--folder` argument is
mandatory and should point to a folder containing at least a `compose.yml`
file. The complete folder will be uploaded to the server (size limits may apply).

### Controlling File Uploads with .scottyignore

You can control which files are uploaded by creating a `.scottyignore` file in your project folder. This file uses gitignore-style patterns to exclude files from being uploaded.

**Pattern Examples:**

```scottyignore
# Ignore log files
*.log

# Ignore build artifacts
target/
node_modules/
dist/

# Ignore environment files
.env
.env.local

# Re-include specific files using ! (negation)
!important.log

# Ignore files in any subdirectory
**/*.tmp
**/.cache/
```

**Common patterns:**

| Pattern | Description |
|---------|-------------|
| `*.log` | Ignore all .log files in any directory |
| `target` | Ignore the target directory and all its contents |
| `!important.log` | Re-include important.log even if *.log is ignored |
| `**/*.tmp` | Ignore .tmp files in any subdirectory |
| `.env*` | Ignore .env, .env.local, etc. |
| `# comment` | Comments (ignored) |

**Note:** The following files are always ignored automatically:
- `.DS_Store` (macOS system file)
- `.git/` directory and its contents

You either need to declare a public service via `--service` or use the
`--app-blueprint` argument (You can get a list of available blueprints with
`scottyctl blueprint:list`). When declaring a public service, you need to
provide a service name and a port. The service name should match a service in the
compose.yml file. The port should be the port the service is listening on.

The `--ttl` argument is optional and will set the lifetime of the app in hours,
days or forever.

You can add basic auth to the app with the `--basic-auth` argument. The argument
should contain a username and a password separated by a colon.

The `--allow-robots` argument will inject a `X-Robots-Tag: noindex` header into
all responses of the app. This will prevent search engines from indexing the app.
(Not supported by all proxies)

The `--destroy-on-ttl` argument will destroy the app after the specified ttl
instead of just stopping it. Suitable for apps that are not expected to be used
for a long time.
Tbhe `--env-file` argument will load environment variables from a file. The file
should contain key-value pairs separated by an equal sign.

You can add custom domains to the app with the `--custom-domain` argument. The
argument should contain a domain and a service name separated by a colon. The
service name should match a service in the compose.yml file.

The `--env-file` argument will load environment variables from a file. The file
should contain key-value pairs separated by an equal sign.

You can add environment variables to the app with the `--env` argument. The
argument should contain a key and a value separated by an equal sign. You can
reference secrets from 1Password with the `OP`-uri-scheme. The value should be
a URL like `op://<connect-instance-name>/<vault-uuid>/<item-uuid>/<field-name>`.
The server needs to be configured accordingly.

You can use a private registry for the images with the `--registry` argument. The
argument should contain the name of the registry. The server needs to be
configured accordingly.

You can add middleware to the app with the `--middleware` argument. The argument
should contain the name of the middleware. Middleware must be in the allow-list in
the server configuration before they can be used. You can specify multiple
middleware by using the `--middleware` argument multiple times. (This is only
supported for traefik)

### Some examples:

```shell
scottyctl --server <SERVER> --access-token <TOKEN> app:create my-nginx-test \
  --folder . \
  --service nginx:80
```

will beam up the current folder to the server and start the nginx service on port 80.

```shell
scottyctl --server <SERVER> --access-token <TOKEN> app:create my-nginx-test \
  --folder . \
  --service nginx:80 \
  --basic-auth user:password \
  --allow-robots \
  --ttl forever
```

will beam up the current folder to the server and start the nginx service on port 80.
It will add basic auth with the username `user` and the password `password` and
won't add a `X-Robots-Tag` header to all responses. The app will run forever.

```shell
scottyctl --server <SERVER> --access-token <TOKEN> app:create my-nginx-test \
  --folder . \
  --service nginx:80 \
  --custom-domain nginx.example.com:nginx
```

will beam up the current folder to the server and start the nginx service on port 80.
The app will be reachable under `http://nginx.example.com`.

## Adopt an app

```shell
scottyctl --server <SERVER> --access-token <TOKEN> app:adopt <APP>
```

This command will adopt an unsupported app. For this to work, the app needs to
be already in the server's app directory. The command will create a `.scotty.yml`
file in the app directory and add the app to the server's database.

Scotty will also try to reuse the existing config from the load balancer and add
that information to the `.scotty.yml` file. It will also dump all found
environment variables into the `.scotty.yml` file.

After adopting an app, it is strongly advised to check the `.scotty.yml` file and
remove any unnecessary information from it and double-check the configuration.

## Destroy an app

```shell
scottyctl --server <SERVER> --access-token <TOKEN> app:destroy <APP>
```

This command will destroy only a supported app. It will stop the app, remove
all ephemeral and persistent data and remove the app from the Scotty server.
It will also delete the used images if they are not used somewhere else.

Caution: This command is irreversible! You might lose data if you run this command.

## List all blueprints

```shell
scottyctl --server <SERVER> --access-token <TOKEN> blueprint:list
```

This will list all available blueprints on the server.

## Get blueprint details

```shell
scottyctl --server <SERVER> --access-token <TOKEN> blueprint:info <BLUEPRINT>
```

This command displays detailed information about a specific blueprint, including its configuration, services, and required parameters.

## Custom Actions

Custom actions allow you to define and execute arbitrary commands on app services. They support an approval workflow for security control.

### Run a custom action

```shell
scottyctl --server <SERVER> --access-token <TOKEN> app:action <APP> <ACTION>
```

Execute an approved custom action. Only actions with `approved` status can be executed.

**Note:** Executing an action requires either `action_read` or `action_write` permission, depending on the action's permission level.

Example:
```shell
# Run a database migration action
scottyctl app:action my-app db:migrate

# Clear application cache
scottyctl app:action my-app cache:clear
```

### List custom actions for an app

```shell
scottyctl --server <SERVER> --access-token <TOKEN> action:list <APP>
```

Lists all custom actions defined for an app, showing their name, description, status, permission level, and creator.

Example:
```shell
scottyctl action:list my-app
```

### Get custom action details

```shell
scottyctl --server <SERVER> --access-token <TOKEN> action:get <APP> <ACTION>
```

Displays detailed information about a specific custom action, including its commands, review status, and metadata.

Example:
```shell
scottyctl action:get my-app db:migrate
```

### Create a custom action

```shell
scottyctl --server <SERVER> --access-token <TOKEN> action:create <APP> <ACTION> \
  --description <DESCRIPTION> \
  --permission <PERMISSION> \
  --command <SERVICE:COMMAND> [--command <SERVICE:COMMAND> ...]
```

Creates a new custom action for an app. The action will be in `pending` status until approved by an administrator with `action_approve` permission.

**Options:**
- `--description` (required): Human-readable description of what the action does
- `--permission`: Required permission level (`action_read` or `action_write`, default: `action_write`)
- `--command`: Command to execute in format `service:command`. Can be specified multiple times.

**Examples:**

Create a write action (modifies state):
```shell
scottyctl action:create my-app db:migrate \
  --description "Run database migrations" \
  --permission action_write \
  --command "web:php artisan migrate" \
  --command "worker:php artisan queue:restart"
```

Create a read-only action (safe to run anytime):
```shell
scottyctl action:create my-app health:check \
  --description "Check application health status" \
  --permission action_read \
  --command "web:php artisan health:check"
```

### Delete a custom action

```shell
scottyctl --server <SERVER> --access-token <TOKEN> action:delete <APP> <ACTION>
```

Removes a custom action from an app.

Example:
```shell
scottyctl action:delete my-app old-action
```

### Action Approval Workflow

Custom actions go through an approval workflow:

1. **Pending**: Newly created actions await approval
2. **Approved**: Action can be executed
3. **Rejected**: Action was rejected and cannot be executed
4. **Revoked**: Previously approved action was revoked
5. **Expired**: Action expired due to TTL (if configured)

See [Admin Commands](#custom-action-approval-admin) for approval workflow management.

## Add a notification service to an app

```shell
scottyctl --server <SERVER> --access-token <TOKEN> notify:add <APP> \
  --service-id <SERVICE_TYPE://SERVICE_ID/CHANNEL|PROJECT_ID/MR_ID>
```

This command will add a notification service to an app. That means scotty will
send a notification for every action on that app to the selected service. The
service needs to be configured on the server.

Currently there are three service types available:
  * `mattermost://SERVICE_ID/CHANNEL`: Send a message to a mattermost channel
  * `gitlab://SERVICE_ID/PROJECT_ID/MR_ID`: Add a comment to a gitlab merge request
  * `webhook://SERVICE_ID`: Send a webhook to a configured URL

## Remove a notification service from an app

```shell
scottyctl --server <SERVER> --access-token <TOKEN> notify:remove <APP> \
  --service-id <SERVICE_TYPE://SERVICE_ID/CHANNEL|PROJECT_ID/MR_ID>
```

This command will remove a notification service from an app. The format of
`SERVICE_ID` is the same as in the `notify:add` command.

## List all notification services of an app

```shell
scottyctl --server <SERVER> --access-token <TOKEN> app:info <APP>
```

For more info, see the help for [`app:info`](http://localhost:8080/cli.html#get-info-about-an-app).

## Authorization Management (Admin Commands)

These commands require `admin_read` or `admin_write` permissions. See the [Authorization documentation](authorization.html) for more details.

### Scopes Management

**List all scopes:**
```shell
scottyctl --server <SERVER> --access-token <TOKEN> admin:scopes:list
```

Lists all authorization scopes with their descriptions and creation timestamps.

**Create a new scope:**
```shell
scottyctl --server <SERVER> --access-token <TOKEN> admin:scopes:create <NAME> <DESCRIPTION>
```

Example:
```shell
scottyctl admin:scopes:create staging "Staging environment applications"
```

### Roles Management

**List all roles:**
```shell
scottyctl --server <SERVER> --access-token <TOKEN> admin:roles:list
```

Lists all roles with their descriptions and associated permissions.

**Create a new role:**
```shell
scottyctl --server <SERVER> --access-token <TOKEN> admin:roles:create <NAME> <DESCRIPTION> <PERMISSIONS>
```

Permissions should be comma-separated. Use `*` for all permissions.

Example:
```shell
# Create a developer role with specific permissions
scottyctl admin:roles:create developer "Developer access" view,manage,shell,logs,create

# Create an admin role with all permissions
scottyctl admin:roles:create admin "Full access" "*"
```

### Assignments Management

**List all user assignments:**
```shell
scottyctl --server <SERVER> --access-token <TOKEN> admin:assignments:list
```

Lists all user-to-role assignments with their assigned scopes.

**Create a new assignment:**
```shell
scottyctl --server <SERVER> --access-token <TOKEN> admin:assignments:create <USER> <ROLE> <SCOPES>
```

Scopes should be comma-separated. Use `*` for all scopes.

Examples:
```shell
# Assign user to developer role in staging scope
scottyctl admin:assignments:create alice@example.com developer staging

# Assign bearer token to admin role across all scopes
scottyctl admin:assignments:create identifier:ci-bot admin "*"

# Assign user to multiple scopes
scottyctl admin:assignments:create bob@example.com operator staging,production
```

**Remove an assignment:**
```shell
scottyctl --server <SERVER> --access-token <TOKEN> admin:assignments:remove <USER> <ROLE> <SCOPES>
```

Example:
```shell
scottyctl admin:assignments:remove alice@example.com developer staging
```

### Permissions Management

**List all available permissions:**
```shell
scottyctl --server <SERVER> --access-token <TOKEN> admin:permissions:list
```

Lists all permissions that can be assigned to roles.

**Test permission for a user:**
```shell
scottyctl --server <SERVER> --access-token <TOKEN> admin:permissions:test <USER> <APP> <PERMISSION>
```

Tests whether a specific user has a particular permission on an app.

Example:
```shell
scottyctl admin:permissions:test alice@example.com my-app manage
```

**Get all permissions for a user:**
```shell
scottyctl --server <SERVER> --access-token <TOKEN> admin:permissions:user <USER>
```

Displays all effective permissions for a user across all scopes.

Example:
```shell
scottyctl admin:permissions:user alice@example.com
```

### Custom Action Approval (Admin) {#custom-action-approval-admin}

These commands require `action_approve` permission and are used to manage the approval workflow for custom actions.

**List pending actions:**
```shell
scottyctl --server <SERVER> --access-token <TOKEN> admin:actions:pending
```

Lists all custom actions awaiting approval across all apps.

**Get action details:**
```shell
scottyctl --server <SERVER> --access-token <TOKEN> admin:actions:get <APP> <ACTION>
```

Displays detailed information about a pending action for review.

Example:
```shell
scottyctl admin:actions:get my-app db:migrate
```

**Approve an action:**
```shell
scottyctl --server <SERVER> --access-token <TOKEN> admin:actions:approve <APP> <ACTION> [--comment <COMMENT>]
```

Approves a pending action, allowing it to be executed.

Example:
```shell
scottyctl admin:actions:approve my-app db:migrate --comment "Reviewed and approved for production"
```

**Reject an action:**
```shell
scottyctl --server <SERVER> --access-token <TOKEN> admin:actions:reject <APP> <ACTION> [--comment <COMMENT>]
```

Rejects a pending action. Rejected actions cannot be executed.

Example:
```shell
scottyctl admin:actions:reject my-app dangerous-action --comment "Security concern: command allows arbitrary file access"
```

**Revoke an action:**
```shell
scottyctl --server <SERVER> --access-token <TOKEN> admin:actions:revoke <APP> <ACTION> [--comment <COMMENT>]
```

Revokes a previously approved action. Use this when an action should no longer be available.

Example:
```shell
scottyctl admin:actions:revoke my-app old-migration --comment "Migration completed, no longer needed"
```

## Shell Completion

Generate shell completion scripts for bash, zsh, fish, or PowerShell:

```shell
scottyctl completion <SHELL>
```

Examples:
```shell
# Bash
scottyctl completion bash > /etc/bash_completion.d/scottyctl

# Zsh
scottyctl completion zsh > ~/.zsh/completion/_scottyctl

# Fish
scottyctl completion fish > ~/.config/fish/completions/scottyctl.fish

# PowerShell
scottyctl completion powershell > scottyctl.ps1
```

After installing the completion script, restart your shell or source the completion file.
