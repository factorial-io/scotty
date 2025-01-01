# scotty

![Tests](https://github.com/factorial-io/scotty/actions/workflows/ci.yml/badge.svg)
![Build](https://github.com/factorial-io/scotty/actions/workflows/release.yml/badge.svg)

## About

**scotty -- yet another micro platform as a service** is a Rust
server providing an API to create, start, stop or destroy a
Docker Compose-based application on your own hardware.

The repo contains two applications:

* `scotty` a Rust-based HTTP server providing an API to talk with the
  service and to start, stop and run Docker Compose-based applications.
  The service provides a user interface at e.g. `http://localhost:21342/`.
  The API is documented at `http://localhost:21342/rapidoc`
* `scottyctl`, a CLI application to talk with the service and execute
  commands from your shell

## Installation

Please have a look at the detailed installation instructions [here](docs/content/installation.md)

## CLI usage

You need to pass the address to the server to the CLI, either by providing
the `--server`-argument or by setting the `SCOTTY_SERVER` env-var.

```shell
scottyctl help
```

will show some help and a list of available commands. You can get help
with `scottyctl help <command>`. A complete list of commands is available [here](docs/content/cli.md)

Make sure to leverage `scottyctl completion $SHELL` to get autocompletion for
your shell.

## Configuring the CLI

The CLI needs only two environment variables to work:
* `SCOTTY_SERVER` the address of the server
* `SCOTTY_ACCESS_TOKEN` the bearer token to use

You can provide the information either via env-vars or by passing the
`--server` and `--access-token` arguments to the CLI.

## Developing/Contributing

We welcome contributions! Please fork the repository, create a
feature branch and submit a pull-request.

* Try to add tests for your bug fixes and features.
* Use conventional commits

### Requirements

To run the server locally you need to have Docker and Docker Compose
installed on your local machine. You also need a recent Rust toolchain.
To get things up and running please start Traefik with:

```shell
cd apps/traefik
docker-compose up -d
```

and then start the server with:

```shell
cargo run --bin scotty  or your preferred way to run a rust binary
```

### Pre-push git hook via [cargo-husky](https://github.com/rhysd/cargo-husky)

This project uses a pre-push git-hook installed by cargo husky. It should be installed automatically.

### Updating the changelog

We are using [git-cliff](https://git-cliff.org) to enforce a changelog. Please update the changelog with
the following command:

```shell
git cliff > changelog.md
```
### Create a new release

We are using `cargo-release` to patch up a new release, this is a typical
command to create a new release:

```shell
cargo release --no-publish alpha -x --tag-prefix ""
```

Adapt to your current needs.
