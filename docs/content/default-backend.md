# Default backend & landing page

Scotty can act as the **default backend** for your load balancer. When a user
visits the domain of an app that is currently *stopped*, the request no longer
fails with a gateway error. Instead the load balancer forwards it to Scotty,
which shows a friendly landing page with a **Start** button. After the user logs
in (if needed) and the app has started, they are redirected back to the original
URL.

This turns stopped review apps and on-demand environments into a self-service
experience: the domain always "works", and visitors can spin the app back up
themselves without ever opening the Scotty dashboard.

## How it works

```
User visits myapp.example.com  (app is stopped, no containers running)
        │
        ▼
Traefik has no route for the domain
        │
        ▼
Catch-all router forwards the request to Scotty (the default backend)
        │
        ▼
Scotty inspects the Host header, finds the app that owns the domain,
and issues a 302 redirect to its own landing page:
        │
        ▼
https://scotty.example.com/landing/myapp?return_url=https://myapp.example.com/...
        │
        ▼
Landing page → [ Start App ] → (OAuth login if needed) → live startup output
        │
        ▼
App is running → redirect back to https://myapp.example.com/...
```

The user only ever sees three states on the landing page:

1. **Stopped** — "This app is not running. [Start App]"
2. **Starting** — live docker-compose output streamed over the WebSocket
3. **Ready** — a short countdown, then an automatic redirect to the original URL

Login happens invisibly between clicking *Start* and the startup output: the user
is bounced through the normal OAuth flow and returns straight to the landing page,
which auto-triggers the start (via an `autostart=true` parameter).

### What Scotty does per app status

The catch-all only forwards requests for domains that Traefik has **no** route for
— which normally means the app is stopped. Scotty double-checks the app status and
responds accordingly:

| App status | Response |
|------------|----------|
| `Stopped` | `302` redirect to the landing page (`/landing/<app>?return_url=...`) |
| `Running` | `503 Service Unavailable` with `Retry-After: 5` — the app is up but routing has not caught up yet (also logged as a possible load balancer misconfiguration) |
| Any other (e.g. starting/creating) | `503 Service Unavailable` — "the application is starting up" |
| Domain belongs to no known app | `404 Not Found` — "No application is configured for this domain" |
| Request is for Scotty's own domain | The Scotty frontend is served as normal |

All landing-related responses are sent with `Cache-Control: no-store` so browsers
and proxies never cache the redirect or the error pages for a stopped app.

## Configuration

Two things are required:

1. **`api.base_url`** — so Scotty knows its own public URL (to distinguish its own
   domain from app domains, and to build the redirect target).
2. **A catch-all router** on your load balancer that forwards otherwise-unmatched
   domains to Scotty.

### 1. Set `api.base_url`

Set the public-facing URL under which Scotty itself is reachable:

```yaml
api:
  base_url: "https://scotty.example.com"
```

Or via environment variable:

```shell
SCOTTY__API__BASE_URL="https://scotty.example.com"
```

Scotty uses this value to:

* Recognise requests for its **own** domain and serve the frontend directly
  instead of trying to redirect them.
* Build the `https://scotty.example.com/landing/<app>?return_url=...` redirect
  target for stopped-app domains.

> If `api.base_url` is not set, Scotty falls back to `api.oauth.frontend_base_url`
> (unless that is still the default `http://localhost:21342`). If neither is
> configured, Scotty cannot tell its own domain apart from app domains: it logs a
> warning and serves every request as the frontend, so **per-app domains will not
> redirect to the landing page**. Always set `api.base_url` in production.

### 2. Configure Traefik as the default backend

Add a **catch-all router** that points at Scotty with the lowest possible
priority. Running apps get their own auto-generated routers whose priority (based
on rule length) is always higher, so a real running app is never shadowed by the
catch-all — only stopped apps (which have no router) fall through to Scotty.

Extend the `labels` of the `scotty` service in your `compose.yml` (see the
[installation guide](installation.md) for the full file):

```yaml
  scotty:
    image: ghcr.io/factorial-io/scotty:main
    # ... volumes, environment, networks as in the installation guide ...
    environment:
      RUST_LOG: info
      SCOTTY__APPS__ROOT_FOLDER: $PWD/apps
      SCOTTY__APPS__DOMAIN_SUFFIX: <TLD>
      SCOTTY__API__BASE_URL: "https://scotty.<TLD>"
    labels:
      - "traefik.enable=true"

      # Scotty's own dashboard — normal, high-priority route
      - "traefik.http.routers.scotty.rule=Host(`scotty.<TLD>`)"
      - "traefik.http.routers.scotty.entrypoints=websecure"
      - "traefik.http.routers.scotty.tls=true"
      - "traefik.http.routers.scotty.tls.certresolver=myresolver"
      - "traefik.http.routers.scotty.service=scotty"

      # Catch-all default backend — any domain without its own (running app)
      # route falls through to Scotty at the lowest possible priority.
      - "traefik.http.routers.scotty-catchall.rule=HostRegexp(`^.+$`)"
      - "traefik.http.routers.scotty-catchall.priority=1"
      - "traefik.http.routers.scotty-catchall.entrypoints=websecure"
      - "traefik.http.routers.scotty-catchall.tls=true"
      - "traefik.http.routers.scotty-catchall.tls.certresolver=myresolver"
      - "traefik.http.routers.scotty-catchall.service=scotty"

      - "traefik.http.services.scotty.loadbalancer.server.port=21342"
```

Notes:

* **Priority `1`** is the lowest possible value. Scotty's own `scotty` router and
  every running app's auto-generated router have a higher default priority, so the
  catch-all only ever matches domains that have no other route.
* The rule uses Traefik v3's `HostRegexp(`^.+$`)`, which matches any host.
* If (as in the reference `compose.yml`) the `web` entrypoint globally redirects
  to `websecure`, plain HTTP requests to a stopped app's domain are upgraded to
  HTTPS first and then hit the catch-all on `websecure`. The catch-all therefore
  only needs to be declared on `websecure`.
* With on-demand certificates (`certresolver`), Traefik will obtain a certificate
  for the stopped app's domain when it is first visited, so the redirect works
  over HTTPS.

#### Using the Traefik file provider

If Scotty does not run as a Traefik-labelled container (for example a
`cargo`-installed binary on the host), declare the same router and service through
the file provider instead:

```yaml
# traefik/dynamic/scotty-catchall.yml
http:
  routers:
    scotty-catchall:
      rule: "HostRegexp(`^.+$`)"
      priority: 1
      entrypoints:
        - websecure
      tls:
        certresolver: myresolver
      service: scotty
  services:
    scotty:
      loadBalancer:
        servers:
          - url: "http://host.docker.internal:21342"
```

## Security

* **Open-redirect protection.** The `return_url` is validated on the frontend
  before the final redirect: it must point to a domain that the app being started
  actually owns. Arbitrary URLs are rejected and the user is offered a link to the
  dashboard instead.
* **Deep-path preservation.** The full path and query string of the original
  request are preserved in `return_url` so the user lands back on the exact page
  they asked for. As a trade-off, if the original URL contained sensitive query
  parameters they will appear in the redirect `Location` header and may be logged
  by intermediate proxies.
* **Permissions.** Starting the app from the landing page uses the normal
  authorization rules — a user still needs `manage` permission on the app's scope,
  or the start will be rejected.

## Troubleshooting

* **App domains are served the Scotty frontend instead of redirecting.**
  `api.base_url` is probably not set (or is still the localhost default). Set it to
  Scotty's public URL. Look for the warning
  *"Neither api.base_url nor api.oauth.frontend_base_url is configured"* in the
  logs.
* **Stopped app domains return a gateway error instead of the landing page.**
  The catch-all router is missing or not matching. Check the Traefik dashboard for
  a `scotty-catchall` router and confirm its priority is lower than the app
  routers.
* **A running app's domain intermittently hits the catch-all (503).**
  Routing has not fully propagated yet — the app just started and Traefik has not
  picked up its labels. This is expected briefly after startup; the `Retry-After`
  header tells clients to retry. If it persists, verify the app's Traefik labels
  and the per-app proxy network (see [Traefik configuration](configuration.md#traefik)).
