<div align="center">
  <img src="docs/content/assets/logo.svg" alt="Scotty" width="324" height="80">
</div>

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

### Shell autocompletion

Make sure to leverage `scottyctl completion $SHELL` to get autocompletion for
your shell, see [here](docs/content/installation.md).

## Configuring the CLI

### Option 1: OAuth Authentication (Recommended)

Use OAuth device flow for secure authentication:

```shell
# Authenticate with OAuth
scottyctl auth:login --server https://localhost:21342

# Use authenticated commands
scottyctl app:list
```

### Option 2: Bearer Token

Bearer tokens are configured on the server with logical identifiers that map to secure tokens. Use environment variables or command-line arguments:

```shell
# Via environment variables
export SCOTTY_SERVER=https://localhost:21342
export SCOTTY_ACCESS_TOKEN=your_secure_bearer_token

# Via command-line arguments
scottyctl --server https://localhost:21342 --access-token your_secure_bearer_token app:list
```

**Security Note**: Server administrators should **never store actual bearer tokens in configuration files**. Instead, use placeholder values in config files and set actual secure tokens via environment variables like `SCOTTY__API__BEARER_TOKENS__ADMIN=your_secure_token`. See the [configuration documentation](docs/content/configuration.md) for security best practices.

## Docker Deployment

### Quick Start with Docker

The Docker image includes only the binaries and non-sensitive configuration files (Casbin model, blueprints). Configuration with secrets must be provided at runtime.

**Option 1: Mount configuration directory (recommended)**
```bash
docker run -d \
  -v /path/to/your/config:/app/config:ro \
  -v /path/to/apps:/app/apps \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -p 21342:21342 \
  scotty:latest
```

**Option 2: Use environment variables**
```bash
docker run -d \
  -e SCOTTY__API__AUTH_MODE=bearer \
  -e SCOTTY__API__BEARER_TOKENS__ADMIN=your-secure-token \
  -e SCOTTY__APPS__DOMAIN_SUFFIX=your-domain.site \
  -p 21342:21342 \
  scotty:latest
```

**Option 3: Docker Compose**
```yaml
services:
  scotty:
    image: scotty:latest
    ports:
      - "21342:21342"
    volumes:
      - ./config:/app/config:ro
      - ./apps:/app/apps
      - /var/run/docker.sock:/var/run/docker.sock
    environment:
      - SCOTTY__API__BEARER_TOKENS__ADMIN=${ADMIN_TOKEN}
    restart: unless-stopped
```

**Important**: Never commit secrets to git. Use environment variables or mount configuration files at runtime. See [config/README.md](config/README.md) for detailed configuration documentation.

## Observability

Scotty includes a comprehensive observability stack with metrics, distributed tracing, and pre-built dashboards for monitoring application health and performance.

### Quick Start

Start the observability stack (Grafana, Jaeger, VictoriaMetrics, OpenTelemetry Collector):

```shell
cd observability
docker-compose up -d
```

Enable telemetry in Scotty:

```shell
SCOTTY__TELEMETRY=metrics,traces cargo run --bin scotty
```

Access the services:
- **Grafana Dashboard**: http://grafana.ddev.site (admin/admin)
- **Jaeger Tracing**: http://jaeger.ddev.site
- **VictoriaMetrics**: http://vm.ddev.site

### What's Monitored

Scotty exports 40+ metrics covering:
- Log streaming (active streams, throughput, errors)
- Shell sessions (active connections, timeouts)
- WebSocket connections and message rates
- Task execution and output streaming
- HTTP server performance by endpoint
- Memory usage (RSS and virtual)
- Application fleet metrics
- Tokio async runtime health

### Documentation

For complete setup instructions, metrics reference, and production deployment guide:

📖 **[Observability Documentation](docs/content/observability.md)**
📖 **[Observability Setup Guide](observability/README.md)**

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
