# Installation

Scotty consists of two parts: The server and the CLI. The server is a rust
application running on a server node providing a REST API to interact with your
applications.

The CLI is a rust-based CLI application running on your local machine or on a
CI/CD pipeline. It interacts with the server to create, start, stop or delete an
app.

## Installation of the server

### using docker-compose

Use the official docker image to run the server. Here's an example configuration
combining scotty with the loadbalancer traefik. The configuration will use ssl-
termination using letsencrypt. Please replace all entries bracketed with `<...>`
with suitable values.

The folder where all your apps live is `/opt/containers/apps`. Please note that
you use the same folder-path for your apps as on your local as within scotty.
Otherwise docker-compose can't find the files, as docker-compose is executed
inside the scotty-container but working on the host.

We are using a dedicated network named `proxy` for the communication of
`traefik ↔ scotty ↔ apps`. If you want to use a different network, do not
forget to adapt the scotty-configuration accordingly.

Do also not forget to create the network with

```shell
docker network create proxy
```

An example docker-compose.yml-file:

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
    image: ghcr.io/factorial-io/scotty:main # or use a dedicated version
    volumes:
      # we need to map the host apps folder to the same path, otherwise the
      # folder mapping wont match for docker compose files of runing apps
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

### Using cargo

You can also install the server with cargo. Please note that you need to have
the rust-toolchain installed on your server. You can install the server with

```shell
cargo install --git https://github.com/factorial-io/scotty.git --bin scotty
```

You then need the config-folder from the repository as a starting point. Place
it on the same level as the executable.

### Using docker only

Use the provided docker-image for best results. Map the directory with
all your docker-composed apps to `/app/apps`.

```shell
docker run \
  -p 21342:21342 \
  -v $PWD/apps:/app/apps \
  -v /var/run/docker.sock:/var/run/docker.sock \
  ghcr.io/factorial-io/scotty:main
```

You might need to map your local config overrides into the container.

## Installation of the cli

### From github

You can download the latest release from the github [releases page](https://github.com/factorial-io/scotty/releases)
Choose the binary for your platform and download it. Make it executable and
place it in your path.

```shell
# Replace version number with latest one.
curl -L https://github.com/factorial-io/scotty/releases/download/v0.1.0-alpha.13/scottyctl-aarch64-apple-darwin.tar.gz -o scottyctl.tar.gz
tar -xvf scottyctl.tar.gz
chmod +x scottyctl
mv scottyctl /usr/local/bin
```

### From source

```shell
cargo install --git https://github.com/factorial-io/scotty.git --bin scottyctl
```

Please note, that you need a installed and working rust toolchain on your local.

### Test installation

Check the installation with

```shell
scottyctl --version
```
