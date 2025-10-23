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
  access_token: "mysecret"
  create_app_max_size: "50M"
```

* `bind_address`: The address and port the server listens on.
* `access_token`: The token to authenticate against the server. This token is
  needed by the clients to authenticate against the server.
* `create_app_max_size`: The maximum size of the uploaded files. The default
  is 50M. As the payload gets base64-encoded, the actual possible size is a
  bit smaller (by ~ 2/3)

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
concatenated with *double underscores*. For example to override the access token
you can set the environment variable `SCOTTY__API__ACCESS_TOKEN`.

Rule of thumb is: If you want to override a key, replace the dots with double
underscores and prefix the key with `SCOTTY__`.

#### Example

| name of value in the config file                  | environment variable                                     |
|---------------------------------------------------|----------------------------------------------------------|
| `debug`                                           | `SCOTTY__DEBUG`                                          |
| `api.access_token`                                | `SCOTTY__API__ACCESS_TOKEN`                              |
| `api.bind_address`                                | `SCOTTY__API__BIND_ADDRESS`                              |
| `docker.registries.example_registry.password`     | `SCOTTY__DOCKER__REGISTRIES__EXAMPLE_REGISTRY__PASSWORD` |
| `apps.domain_suffix`                              | `SCOTTY__APPS__DOMAIN_SUFFIX`                            |
| `load_balancer_type`                              | `SCOTTY__LOAD_BALANCER_TYPE`                             |
| `traefik.network`                                 | `SCOTTY__TRAEFIK__NETWORK`                               |

Scotty will print out the resolved configuration on startup, so you can check
for any errors.
