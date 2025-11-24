# Architecture

## Overview

Scotty provides a simple REST API to interact with your apps. For that, it
traverses a directory structure on the server and reads the `compose.yml`
files in each found directory. If the same directory contains a `.scotty` file,
it reads the settings for that app from that file.

Scotty uses the information from the `.scotty.yml` file to create a
`compose.override.yml` file to instruct a load balancer on how to reach
the exposed services of the app. Scotty does not touch any other files in the
directory besides the `compose.override.yml` and `.scotty.yml` files.

Scotty also tracks how long an app has been running and stops it after
a given lifetime. It can add basic auth to the app and prevent
robots from indexing the app if needed.

## The anatomy of an app

Every folder with a `compose.yml` file is considered an app. The
folder needs to reside in the apps directory of Scotty (see configuration).

Every app has a unique name which derives from the folder the compose.yml
is in. The name is used to identify the app in the UI and CLI. An app has
numerous services, which are defined in the compose.yml file. Some of the
services are exposed to the public, some are not. Every service gets a unique
hostname, which is derived from the app name and service name, but this can
be overridden in the `.scotty.yml` file or while creating a new app.

Ideally the docker-compose references pre-built docker images, but it can also
build images on the fly from Dockerfiles.

Example layout of the apps directory:
```
.
├── feat-preview-apps-using-scotty-test
│   ├── compose.override.yml
│   └── compose.yml
├── main-test-app
│   ├── compose.override.yml
│   ├── compose.yml
│   ├── private
│   ├── redis.conf
│   └── web
└── nginx-test
    ├── compose.override.yml
    ├── compose.yml
    └── html
        ├── index.html
        └── static
            ├── app.5a03541a7bda648594c1.js
            └── app.b77a84840a9e3574131f2ed36a54aa86.css
```

## Types of apps

Scotty does not support all possible docker-compose settings. `compose.yml`
is validated and categorized into three types: *owned*, *supported* and
*unsupported*:

Unsupported features are:
* Exposing ports directly, as this might conflict with other running apps
* Using environment-variable expansion inside the compose.yml file. This is
  not supported, as Scotty can't know the values of the environment variables
  at runtime. You can adopt these types of apps manually and provide the values
  for the environment variables in the `.scotty.yml` file.

Scotty will also try to handle apps found in the apps directory or its
subdirectories. That means you can have other apps running on the server which
won't be visible or interfered with by Scotty.

### Owned apps

Owned apps are either created by Scotty, or adopted manually. Scotty is allowed
to manage the whole lifecycle of the app, even destroying the app and all
its data.

### Supported apps

Supported apps are docker-compose-based applications which can be handled by
Scotty. They do not have any side-effects in their docker-compose file like
exposed ports or needed environment variables. Scotty can handle the complete
lifecycle of the app, but won't allow destroying the app and all its data.

### Unsupported apps

Unsupported apps are docker-compose-based applications which need environment
variables to interact with `docker-compose` or are exposing ports directly.
Scotty won't touch these apps but will show them in the UI and CLI. You
can use the cli-command `app:adopt` to make the app compatible with Scotty.

### Blueprints

Owned apps can adopt blueprints to provide additional functionality. Blueprints
store common tasks to execute on certain liftime events like `app:create`,
`app:run` or `app:destroy`.

These scripts are stored in a YML-file in the `blueprints` directory of Scotty.
The scripts are executed in the running service container of the app. Common
tasks could be, for example, running the deploy command for Drupal applications,
or clearing the cache for other apps.

Blueprints can be provided when creating a new app via `app:create`. A list of
blueprints can be get by running `blueprint:list`.

## Server-Architecture

Scotty traverses a dedicated folder on the server to find possible apps. If
there is a folder with a valid compose.yml file, Scotty will add the app
to its internal database. If there is a corresponding `.scotty.yml` file, Scotty
will also read the settings for that particular app.

When Scotty creates a new app, it will save the settings in the `.scotty.yml` file
and create a `compose.override.yml` file to instruct the load balancer on
what domain should be used to reach each public service of an app.

### Overview

![Server Architecture](assets/architecture-diagram.svg)

Scotty works well with the following load balancers:

### [Traefik](https://traefik.io)

Scotty will create the necessary labels for Traefik to route the traffic to the
public services of each app. Depending on the settings, Scotty will also create
configuration to enable basic auth or to prevent robots from indexing the app.

An example `compose.override.yml` file for Traefik:

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

The traefik implementation also supports custom middlewares. Please declare them
in the `config` directory. Then you can refer to the listed middlewares in the
`app:create`-command and the `--middleware`-option.

### [Haproxy-Config](https://github.com/factorial-io/haproxy-config)

Scotty supports the legacy setup called haproxy-config. It will create the
necessary compose.override.yml to instruct haproxy-config to route the
traffic to the public services of each app. Haproxy-config does not support
preventing robots from indexing the app. The support for haproxy-config won't be
continued in the future, as haproxy-config is deprecated.

An example `compose.override.yml` file for haproxy-config:

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

Best practice is to have a wildcard domain pointing to the server where Scotty
is running and giving Scotty a subdomain to manage the apps. This way Scotty can
create new domains for apps without further DNS configuration being necessary. It
is also advised to use Let's Encrypt to provide SSL certificates for the domains
and apps. Both proxy types support Let's Encrypt.

An example setup:

```
apps.example.com   A     1.2.34
*.apps.example.com CNAME apps.example.com.
```

Then you can assign scotty.apps.example.com to Scotty and let Scotty manage the
apps.

## Caveats and pitfalls

* If you are running the Scotty server inside a Docker container, you need
  to make sure that the path to the apps folder is the same on both the host and
  the Docker container, as Scotty is using docker-compose to manage the apps.
  Otherwise mounted binds of your apps would not work as expected, since bind
  mount paths need to be identical on the host and container.
  It's a bit complicated, but as a rule of thumb: when using Scotty, make sure to
  mount the apps folder to the same path!
