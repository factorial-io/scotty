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

### Docker

Use the provided docker-image for best results. Map the directory with
all your docker-composed apps to `/app/apps`.

```shell
docker run \
  -p 21342:21342 \
  -v $PWD/apps:/app/apps \
  -v /var/run/docker.sock:/var/run/docker.sock \
  ghcr.io/factorial-io/scotty:main
```

You can then visit the docs at http://localhost:21342/rapidocs

To run the cli use

```shell
docker run -it ghcr.io/factorial-io/scotty:main /app/scottyctl
```

If you are running the server also locally via docker, you need to adapt the
`--server` argument, e.g.

```shell
docker run -it ghcr.io/factorial-io/scotty:main \
  /app/scottyctl \
  --server http://host.docker.internal:21342 \
  list
```

### docker compose

Scotty works nicely with traefik. Here's an example docker-compose file to
spin up both services:

```yaml
services:
  traefik:
    image: "traefik:v3.1"
    container_name: "traefik"
    command:
      - "--log.level=DEBUG"
      - "--api.insecure=true"
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--providers.docker.network=proxy"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.web.http.redirections.entrypoint.to=websecure"
      - "--entrypoints.web.http.redirections.entrypoint.scheme=https"
      - "--entryPoints.websecure.address=:443"
      - "--certificatesresolvers.myresolver.acme.tlschallenge=true"
      #- "--certificatesresolvers.myresolver.acme.caserver=https://acme-staging-v02.api.letsencrypt.org/directory"
      - "--certificatesresolvers.myresolver.acme.email=<YOUR-LETSENCRYPT-MAIL@ADDRESS>"
      - "--certificatesresolvers.myresolver.acme.storage=/letsencrypt/acme.json"
    ports:
      - "80:80"
      - "443:443"
      - "8080:8080"
    volumes:
      - "./letsencrypt:/letsencrypt"
      - "/var/run/docker.sock:/var/run/docker.sock:ro"
    networks:
      - default
      - proxy
    restart: unless-stopped
    labels:
      traefik.enable: true
      traefik.http.routers.traefik_https.rule: Host(`traefik.<TLD>`)
      traefik.http.routers.traefik_https.entrypoints: websecure
      traefik.http.routers.traefik_https.tls: true
      traefik.http.routers.traefik_https.tls.certResolver: myresolver
      traefik.http.routers.traefik_https.service: api@internal
      traefik.http.routers.traefik_https.middlewares: basic-auth-global
      traefik.http.middlewares.basic-auth-global.basicauth.users: traefik:$$2y$$05$$OjZDsiX5v1NcqHmfsK2AqePaZ87SNNXDVve9wShlKeZ9KMe1vvD/W

  scotty:
    image: ghcr.io/factorial-io/scotty:main
    volumes:
      # we need to map the host apps folder to the same path, otherwise the folder mapping wont match for
      # docker compose files of runing apps
      - /opt/containers/apps:/opt/containers/apps
      - /var/run/docker.sock:/var/run/docker.sock
    environment:
      RUST_LOG: info
      SCOTTY__APPS__ROOT_FOLDER: /opt/containers/apps
      SCOTTY__APPS__DOMAIN_SUFFIX: <TLD>
    networks:
      - default
      - proxy
    restart: unless-stopped
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.scotty.rule=Host(`scotty.<TLD>`)"
      - "traefik.http.routers.scotty.entrypoints=websecure"
      - "traefik.http.routers.scotty.tls=true"
      - "traefik.http.routers.scotty.tls.certresolver=myresolver"
      - "traefik.http.routers.service=scotty"
      - "traefik.http.services.scotty.loadbalancer.server.port=21342"
networks:
  proxy:
    external: true
```

To start the services run

```shell
docker network create proxy
docker compose up -d
```

Set the traefik specific config in the `config/local.yaml`:

```yaml
traefik:
  network: "proxy"
  use_tls: true
  certresolver: "myresolver"
```

### Install native apps

#### from github release

Download the latest release from the [releases page](https://github.com/factorial-io/scotty/releases),
make it executable  and move it to a location in your path.


#### from source

For now you can build the apps either by checking out the repo and running
`cargo build` or if you are only interested in the executables you can also
use

```shell
# for the cli
cargo install --git https://github.com/factorial-io/scotty.git --bin scottyctl
# for the server
cargo install --git https://github.com/factorial-io/scotty.git --bin scotty
```

## CLI usage

You need to pass the address to the server to the cli, either by providing
the `--server`-argument or by setting the `SCOTTY_SERVER` env-var.

```shell
scottyctl help
```

will show some help and a list of available commands. You can get help
with `scottyctl help <command>`

Here's a short list of avaiable commands

* `scottyctl list` will list all apps and their their urls and states
* `scottyctl run <app_name>` will start and run the named app
* `scottyctl stop <app_name>` will stop the named app
* `scottyctl purge <app_name>` will remove runtime files for the named
  app (similar to `docker-compose rm`)
* `scottyctl create` Create a new app
* `scottyctl destroy` Destroy a managed app
* `scottyctl info` Display some info about the app

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
