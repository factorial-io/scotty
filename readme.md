# scotty

## About

**scotty -- yet another feature based deployment service** is a rust
server providing an api to create, start, stop or destroy a
docker-composed-based application.

The repo contains two applications:

* `scotty` a rust based http-server providing an API to talk with the
  service and to start, stop and run docker-composed based applications
  The service provides a ui at e.g. `http://localhost:21342/`. the api is documented at `http://localhost:21342/rapidoc`
* `scottyctl` a cli application to talk with the service and execute
  commands from your shell

## Installation

### Docker

Use the provided docker-image for best results. Map the directory with all your docker-composed apps to `/app/apps`.

```shell
docker run \
  -p 21342:21342 \
  -v $PWD/apps:/app/apps \
  -v /var/run/docker.sock:/var/run/docker.sock \
  registry.factorial.io/administration/scotty/scotty:main
```

You can then visit the docs at http://localhost:21342/rapidocs

To run the cli use

```shell
docker run -it registry.factorial.io/administration/scotty/scotty:main /app/scottyctl
```

If you are running the server also locally via docker, you need to adapt the --server argument, e.g.

```shell
docker run -it registry.factorial.io/administration/scotty/scotty:main \
  /app/scottyctl \
  --server http://host.docker.internal:21342 \
  list
```

### Install native apps

Currently, we do not the apps in the ci, this might happen in a later state. You need the rust tooling on your local.

For now you can build the apps either by checking out the repo and running `cargo build` or
if you are only interested in the executables you can also use

```shell
cargo install --git ssh://git@source.factorial.io/administration/scotty.git --bin scottyctl # for the cli
cargo install --git ssh://git@source.factorial.io/administration/scotty.git --bin scotty # for the server
```

## CLI usage

You need to pass the address to the server to the cli, either by providing the `--server`-argument or by setting the `SCOTTY_SERVER` env-var.

```shell
scottyctl help
```

will show some help and a list of available commands. You can get help with `scottyctl help <command>`

Here's a short list of avaiable commands

* `scottyctl list` will list all apps and their their urls and states
* `scottyctl run <app_name>` will start and run the named app
* `scottyctl stop <app_name>` will stop the named app
* `scottyctl purge <app_name>` will remove runtime files for the named app (similar to `docker-compose rm`)
* `scottyctl create` Create a new app
* `scottyctl destroy` Destroy a managed app
* `scottyctl info` Display some info about the app
