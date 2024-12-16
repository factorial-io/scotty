# Configuration

Configuration on the server side is done using some toml files inside the
config folder, see below.

The CLI does not have any configuration. All it needs to know is the URL of the
server and the api token to authenticate against the server.

## the CLI

Run `scottyctl` with the follwinng options:

```shell
scottyctl --server <SERVER> --token <TOKEN>
scottyctl --server https://loclahost:21342 --token my-secret
```

You can also set the environment variables `SCOTTY_SERVER` and
`SCOTTY_ACCESS_TOKEN` to store the server and token for the CLI.

To check if the server and access-token works, run the command `app:list`:

```shell
scottyctl --server https://loclahost:21342 --token my-secret app:list
```

## the server

The server has a bunch of configuration files in a folder named `config` on the
same level as the binary. It supports overring specific configuration entries
via env-vars or by entire files. Best practice is to setup your app in
`config/local.yaml` and pass sensitive data via env-vars.

We'll describe in this file all sections of the server configuration:o

### global settings

```yaml
debug: false
telemetry: None
frontend_directory: ./frontend/build
```
* `debug`: If set to true, the server will log more information. The default is
  false.
* `telemetry`: The telemetry backend to use. The default is `None`. Possible values
  are `None`, `traces` and `metrics`. Please set also the Opentelmetry endpoint
  where to deliver the traces or metrics. (Use this setting only for debugging)
* frontend_directory: The directory where the frontend is located. The default is
  `./frontend/build`. If you want to use a different frontend, you can set the
  directory here. All files in the directory are served by scotty as static files
  from `/`.

### the API

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

### the scheduler

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

## Apps

```
apps:
  domain_suffix: "ddev.site"
  root_folder: "./apps" # Path to the folder where the apps are stored
```

* `domain_suffix` The suffix for auto-generated domains. Set this to the domain
  you want to use for your apps. App-domains are constructed by the service- and
  the app-name, when a custom domain is not provided.
