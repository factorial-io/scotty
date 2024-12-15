# scotty

Current release: 0.1.1

## About

**scotty -- yet another micro platform as a service** is a rust
server providing an api to create, start, stop or destroy a
docker-composed-based application on your own hardware.

The repo contains two applications:

* `scotty` a rust based http-server providing an API to talk with the
  service and to start, stop and run docker-composed based applications
  The service provides a user interface at e.g. `http://localhost:21342/`.
  the api is documented at `http://localhost:21342/rapidoc`
* `scottyctl`, a cli application to talk with the service and execute
  commands from your shell

## Installation

Please have a look at the detailed installation instructions [here](docs/content/installation.md)

## CLI usage

You need to pass the address to the server to the cli, either by providing
the `--server`-argument or by setting the `SCOTTY_SERVER` env-var.

```shell
scottyctl help
```

will show some help and a list of available commands. You can get help
with `scottyctl help <command>`

Here's a short list of avaiable commands

* `scottyctl app:list` will list all apps and their their urls and states
* `scottyctl app:run <app_name>` will start and run the named app
* `scottyctl app:stop <app_name>` will stop the named app
* `scottyctl app:purge <app_name>` will remove runtime files for the named
  app (similar to `docker-compose rm`)
* `scottyctl app:create` Create a new app
* `scottyctl app:destroy` Destroy a managed app
* `scottyctl app:adopt <app_name>` adopts a legacy app, so it can be
  controlled by scotty. Please note that most likely you need to adjust the
  created `.scotty.yml` file to match your needs.
* `scottyctl app:info` Display some info about the app
* `scottyctl notify:add` Adds a new service to notify on app changes
* `scottyctl notify:remove` Removes a notification to a service

## Configuring the server

You'll find all configuration options in `config/detault.yaml`. Create a
`config/local.yaml` and override the parts you want to change. You can
override the config also by setting environment variables following the
pattern `SCOTTY__GROUP__KEY` e.g. `SCOTTY__API__BIND_ADRESS=0.0.0.0:80`.

If you use scotty using docker, then make sure, that you use the same
path for the apps as on the root host, otherwise relative paths in
the docker-compose files will not work. So if your apps are located in
`/srv/apps` on the host, then you need to mount `/srv/apps` to
`/srv/apps` and adjust the config-file accordingly.

To use the api you need to add a bearer token to your requests. The
bearer token can be set in your configuration (`api.access_token`) or
also via an environment variable, e.g. `SCOTTY__API__ACCESS_TOKEN`.

It is advised to protect the service with a bearer token, as it gives
its users full access to docker and docker-compose.

For a future version it is planned to introduce JWTs and SSO.

## Configuring the cli

The cli needs only two environment variables to work:
* `SCOTTY_SERVER` the address of the server
* `SCOTTY_ACCESS_TOKEN` the bearer token to use

You can provide the information either via env-vars or by passing the
`--server` and `--access-token` arguments to the cli.

## What is the server doing?

* It provides a REST-Api to administer the lifecycle of apps located in the
  `/apps`-folder
* It scans the apps-folder on a regular basis and updates the states of the apps
* it also checks if apps are running longer than their configured TTL and
  kill them, if necessary
* the server provides a svelte-based UI, so you can interact with the API
  from the browser. You need to log in using the same access-token
* It allows you to create new apps based on a bunch of files and a docker-
  compose.yml-file. It will automtically create a `docker-compose.override.yml`
  file for your app so it can be reached from the outside.

  Currently there is support for two reverse-proxies:
  * [traefik](https://traefik.io/traefik/) -- scotty will create the necessary labels to instruct traefik
    to forward traffic to the app
  * [haproxy_config](https://github.com/factorial-io/haproxy-config) -- scotty will create the necessary environment variables
    to forward traffic to the app (legacy)

  When you create a new app you provide a list of public services,
  which then will be used as domain-names for the reverse-rpoxy
  configuration.

## Notifications

Scotty supports notifications to other services, e.g. Gitlab, Mattermost or
via webhooks. Notifications recipients need to be configured on the server
side, but `scottyctl` can provide parameters to steer the delivery, e.g.
the channel-name for mattermost or the merge-request-id for gitlab.

Here's a config-sample-snippet for mattermost (Create an incoming webhook
and note down the hook_id):

```yaml
notification_services:
  our-mattermost:
    type: mattermost
    host: https://chat.example.com
    hook_id: xxx # Override with SCOTTY__NOTIFICATION_SERVICES__OUR_MATTERMOST__HOOK_ID
```
To enable notifications for an app, run

```bash
scottyctl notify:add <APP> --service-id mattermost://our-mattermost/my-custom-channel
```

Similar for gitlab (create a personal access token and note it down):


```yaml
notification_services:
  our-gitlab:
    type: gitlab
    host: https://our.gitlab.com
    token: xxx # Override with SCOTTY__NOTIFICATION_SERVICES__OUR_GITLAB__TOKEN
```
To enable notifications for an app, run

```bash
scottyctl notify:add <APP> --service-id gitlab://our-gitlab/my-group/my-project/3
```

This will add notes to the MR 3 of that particular project.


## Developing/ contributing

We welcome contributions! Please fork the repository, create a
feature branch and submit a pull-request.

* Try to add tests for your bug fixes and features.
* Use conventional commits

### Requirements

To run the server locally you need to have docker and docker-compose
installed on your local. You also need a recent rust toolchain.
To get things up and running please start traefik with

```shell
cd apps/traefik
docker-compose up -d
```

and then start the server with

```shell
cargo run --bin scotty  or your preferred way to run a rust binary
```

### Pre-push git hook via [cargo-husky](https://github.com/rhysd/cargo-husky)

This project uses a pre-push git-hook installed by cargo husky. It shoud be installed automatically.

### Updating the changelog

We are using [git-cliff](https://git-cliff.org) to enforce a changelog. Please update the changelog with
the following command

```shell
git cliff > changelog.md
```
### Create a new release

We are using `cargo-release` to patch up a new release, this is a typical
command to create a new release:

```shell
cargo release --no-publish alpha -x
```

Adapt to your current needs.
