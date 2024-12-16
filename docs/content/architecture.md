# Architecture

## Overview

Scotty is providing a simple REST API to interact with your apps. For that, it
traverses a directory structure on the server and reads the `docker-compose.yml`-
files in each found directory. If the same directory contains a `.scotty`-file,
it reads the settings for that app from that file.

Scotty is using the information from the `.scotty.yml`-file to create a
`docker-compose.override.yml`-file to instruct a load balancer on how to reach
the exposed services of the app. Scotty does not touch any other files in the
directory besides the `docker-compose.override.yml` and the `.scotty.yml`-file.

Scotty keeps also track on how long an app is already running and stops it after
a given lifetime. It also adds basic auth to the app and prevents
robots from indexing the app if needed.

## The anatomy of an app

Every folder with a `docker-compose.yml`-file is considered as an app. The
folder needs to reside in the apps-directory of scotty (see configuration).

Every app has a unique name which derives from the folder the docker-compose.yml
is in. The name is used to identify the app in the UI and the CLI. An app has
numerous services, which are defined in the docker-compose.yml file. Some of the
services are exposed to the public, some are not. Every service gets a a unique
hostname, which is derived from the app-name and the service-name, but this can
be overridden in the `.scotty.yml`-file or while creating a new app.

Ideally the docker-compose references pre-built docker images, but it can also
build images on the fly from Dockerfiles.

Example layout of the apps-directory:
```
.
├── feat-preview-apps-using-scotty-test
│   ├── docker-compose.override.yml
│   └── docker-compose.yml
├── main-test-app
│   ├── docker-compose.override.yml
│   ├── docker-compose.yml
│   ├── private
│   ├── redis.conf
│   └── web
└── nginx-test
    ├── docker-compose.override.yml
    ├── docker-compose.yml
    └── html
        ├── index.html
        └── static
            ├── app.5a03541a7bda648594c1.js
            └── app.b77a84840a9e3574131f2ed36a54aa86.css
```

## Types of apps

Scotty does not support all possible docker-compose settings. `docker-compose.yml`
is validated and categorized into three types: *owned*, *supported* and
*unsupported*:

Unsupported features are:
* Exposing ports directly, as this might conflict with other running apps
* Using environment-variables expansion inside the docker-compose.yml. This is
  not supported, as scotty can't know the values of the environment-variables
  at runtime. You can adopt these types of apps manually and provide the values
  for the environment-variables in the `.scotty.yml`-file.

### Owned apps

Owned apps are either created by scotty, or adopted manually. Scotty is allowed
to manage the whole lifecycle of the app, even destroying the app and all
its data.

### Supported apps

Supported apps are docker-compose-based applications, which can be handled by
scotty. They do not have any side-effects in their docker-compose file like
exposed ports or needed environment-variables. Scotty can handle the complete
life-cycle of the app, but won't allow destroying the app and all its data.

### Unsupported apps

Unsupported apps are docker-compose-based applications, which needs environment-
variables to interact with `docker-compose` or are exposing ports directly.
Scotty won't touch these apps, but will show them in the UI and CLI, but you
can't interact with them. You can use the cli-command `app:adopt` to make the
app compatible with scotty.

## Server-Architecture

Scotty traverses a dedicated folder on the server to find possible apps. If
there is a folder with a valid docker-compose.yml file, scotty will add the app
to its internal database. If there is a corresponding `.scotty.yml`-file, scotty
will read also the setting for that particular app.

When soctty creates a new app, it will save the settings in the `.scotty.yml`-file
and creates a `docker-compose.override.yml`-file to instruct the load balancer on
what domain should be used to reach each public service of an app.

### Overview

![Server Architecture](assets/architecture-diagram.svg)

Scotty works well with the following load balancers:

### [Traefik](https://traefik.io)

Scotty will create the necessary labels for traefik to route the traffic to the
public services of each app. Depending on the settings scotty will also create
configuration to enable basic auth or to prevent robots from indexing the app.

An example `docker-compose.override.yml`-file for traefik:

```yaml
services:
  nginx:
    labels:
      traefik.http.routers.nginx--nginx-again.middlewares: nginx--nginx-test--robots
      traefik.http.routers.nginx--nginx-again.rule: Host(`nginx.nginx-test.example.com`)
      traefik.enable: 'true'
      traefik.http.services.nginx--nginx-again.loadbalancer.server.port: '80'
      traefik.http.routers.nginx--nginx-again.tls: 'true'
      traefik.http.routers.nginx--nginx-again.tls.certresolver: myresolver
      traefik.http.middlewares.nginx--nginx-again--robots.headers.customresponseheaders.X-Robots-Tags: none, noarchive, nosnippet, notranslate, noimageindex
    environment: {}
    networks:
    - default
    - proxy
networks:
  proxy:
    external: true
```

### [Haproxy-Config](https://github.com/factorial-io/haproxy-config)

Scotty supports the legacy setup called haproxy-config. It will create the
necessary docker-compose.override.yml to instruct haproxy-config to route the
traffic to the public services of each app. Haproxy-config does not support
preventing robots from indexing the app. The support for haproxy-config won't be
continued in the future, as haprox-config is deprecated.

An example `docker-compose.override.yml`-file for haproxy-config:

```yaml
services:
  nginx:
    environment:
      VHOST: nginx.test-nginx.example.com
      HTTPS_ONLY: '1'
      VPORT: '80'
      HTTP_AUTH_USER: nginx
      HTTP_AUTH_PASS: nginx
```

## Domain-Setup

Best practice is to have a wildcard domain pointing to the server where scotty
is running and giving scotty a subdomain to manage the apps. This way scotty can
create new domains for apps, without further DNS-configuration is necessary. It
is also advised to use letsencrypt to provide ssl certificates for the domains
and apps. Both proxy-types do support letsencrypt.

An example setup:

```
apps.example.com   A     1.2.34
*.apps.example.com CNAME apps.example.com.
```

Then you can assign scotty.apps.example.com to scotty and let scotty manage the
apps.
