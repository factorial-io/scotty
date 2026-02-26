# Scotty Landing Page for Stopped Apps

## Problem

When a user visits an app URL (e.g., `myapp.example.com`) and the app is not running, Traefik has no backend to route to -- the request fails with a gateway error. There is no way for the user to start the app without going to the Scotty dashboard first.

## Goal

Provide a seamless experience: when a stopped app's domain is visited, the user sees a friendly landing page with a prominent "Start" button. Clicking it triggers login (if needed), then transitions to a status page showing startup progress, and finally redirects back to the original app URL.

## User Flow

```
User visits myapp.example.com
        |
        v
App is stopped, no containers running
        |
        v
Traefik has no route -> falls back to default backend (Scotty)
        |
        v
Scotty receives request, inspects Host header
        |
        v
Scotty finds app "myapp" owns domain "myapp.example.com"
        |
        v
302 redirect to: scotty.example.com/landing/myapp?return_url=https://myapp.example.com
        |
        v
    +--------------------------+
    |     LANDING PAGE         |
    |                          |
    |  "myapp" is not running  |
    |                          |
    |    [ Start App ]         |
    |                          |
    +--------------------------+
        |
        | user clicks "Start"
        v
    +-- not logged in? --------+
    |                          |
    |  OAuth login flow        |
    |  (standard Scotty auth)  |
    |                          |
    |  returns to landing page |
    |  with login complete     |
    +------+-------------------+
           |
           | now logged in, auto-trigger start
           v
    +--------------------------+
    |     STATUS PAGE          |
    |                          |
    |  Starting "myapp"...     |
    |                          |
    |  [==========>    ] 70%   |
    |                          |
    |  > Pulling images...     |
    |  > Creating containers   |
    |  > Starting services     |
    |                          |
    +--------------------------+
        |
        | task completes
        v
    +--------------------------+
    |                          |
    |  App is ready!           |
    |  Redirecting...          |
    |                          |
    +--------------------------+
        |
        v
Redirect to myapp.example.com
Traefik now routes to the running app
```

### Key UX principle

The user only ever sees **three states**:

1. **Landing** -- "This app is stopped. [Start]"
2. **Starting** -- spinner, progress, task output
3. **Redirect** -- "Ready! Redirecting..."

Login is invisible friction -- it happens between clicking "Start" and seeing the status page. The user clicks "Start", gets bounced through OAuth if needed, and lands directly on the status page with the app already starting.

## Architecture

### 1. Traefik Catch-All Default Backend

**What:** Configure Traefik to route unmatched domains to Scotty as a fallback.

**How:** Add a catch-all router with the lowest possible priority so that any domain without an explicit route (i.e., a stopped app whose containers are down) is forwarded to Scotty.

**Option A -- Scotty runs as a Docker container with Traefik labels:**

```yaml
# In Scotty's docker-compose.yml (production deployment)
services:
  scotty:
    image: scotty:latest
    labels:
      traefik.enable: "true"
      # Catch-all rule with lowest priority
      traefik.http.routers.scotty-catchall.rule: "PathPrefix(`/`)"
      traefik.http.routers.scotty-catchall.priority: "1"
      traefik.http.routers.scotty-catchall.entrypoints: "web"
      traefik.http.services.scotty-catchall.loadbalancer.server.port: "21342"
      # Scotty's own domain (higher priority, normal routing)
      traefik.http.routers.scotty.rule: "Host(`scotty.example.com`)"
      traefik.http.routers.scotty.entrypoints: "web"
```

**Option B -- Traefik file provider (for non-containerized Scotty):**

```yaml
# traefik/dynamic/scotty-catchall.yml
http:
  routers:
    scotty-catchall:
      rule: "PathPrefix(`/`)"
      priority: 1
      service: scotty
      entrypoints:
        - web
  services:
    scotty:
      loadBalancer:
        servers:
          - url: "http://host.docker.internal:21342"
```

**Key detail:** Priority `1` is the lowest possible. Running app containers get auto-generated Traefik routers with default priority (based on rule length), which is always higher.

### 2. Domain-to-App Lookup

**What:** Add a method to resolve which app owns a given domain.

**Where:** `scotty-core/src/apps/shared_app_list.rs`

```rust
/// Look up an app by one of its domains.
pub async fn find_app_by_domain(&self, domain: &str) -> Option<AppData> {
    let apps = self.apps.read().await;
    for app in apps.values() {
        if let Some(settings) = &app.settings {
            for service in &settings.public_services {
                if service.domains.contains(&domain.to_string()) {
                    return Some(app.clone());
                }
                let auto_domain = format!("{}.{}", service.service, settings.domain);
                if auto_domain == domain {
                    return Some(app.clone());
                }
            }
        }
        for container in &app.services {
            if container.domains.contains(&domain.to_string()) {
                return Some(app.clone());
            }
        }
    }
    None
}
```

### 3. Host Header Detection and Redirect (Backend)

**Where:** Replace the fallback handler in `scotty/src/api/router.rs`

When a request arrives with a Host header that is not Scotty's own domain, look up the app and issue a 302 redirect:

```rust
async fn landing_or_frontend_handler(
    Host(hostname): Host,
    State(state): State<SharedAppState>,
    uri: axum::http::Uri,
) -> Response<Body> {
    if is_scotty_domain(&state, &hostname) {
        return serve_embedded_file(uri).await;
    }

    if let Some(app) = state.app_list.find_app_by_domain(&hostname).await {
        let scheme = if state.settings.traefik.use_tls { "https" } else { "http" };
        let return_url = format!("{}://{}{}", scheme, hostname, uri.path());
        let scotty_base = &state.settings.api.base_url;

        let redirect_url = format!(
            "{}/landing/{}?return_url={}",
            scotty_base,
            urlencoding::encode(&app.name),
            urlencoding::encode(&return_url),
        );

        return Response::builder()
            .status(StatusCode::FOUND)
            .header("Location", redirect_url)
            .body(Body::empty())
            .unwrap();
    }

    // Unknown domain
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("content-type", "text/html")
        .body(Body::from("<h1>App not found</h1>"))
        .unwrap()
}
```

### 4. Landing Page (SvelteKit Route)

**Route:** `/landing/[slug]?return_url=...`

**Why SvelteKit:** The redirect brings the user to Scotty's domain, so the full SvelteKit app is available -- existing auth, API client, WebSocket, DaisyUI styling all work out of the box.

The page is a single component with three visual states, driven by a simple state machine:

#### State: `idle` (initial)

```
+--------------------------------------+
|                                      |
|         "myapp" is stopped           |
|                                      |
|   This app is currently not running. |
|   Click below to start it.          |
|                                      |
|          [ Start App ]               |
|                                      |
+--------------------------------------+
```

- Shows app name (from URL param `slug`)
- Shows the domain the user came from (from `return_url`)
- Prominent "Start App" button

**When user clicks "Start":**
1. Check if user is authenticated (token exists and is valid)
2. **If not authenticated:** Redirect to OAuth login with a `redirect_uri` that comes back to this same landing page with an `autostart=true` param:
   ```
   /oauth/authorize?redirect_uri=/landing/myapp?return_url=...&autostart=true
   ```
3. **If authenticated:** Transition to `starting` state, call `GET /api/v1/authenticated/apps/run/{appName}`

**When page loads with `autostart=true`** (returning from OAuth):
- User just completed login, auto-trigger the start action immediately
- Skip the idle state, go straight to `starting`

#### State: `starting`

```
+--------------------------------------+
|                                      |
|      Starting "myapp"...             |
|                                      |
|      [============>       ]          |
|                                      |
|   > Pulling images...          done  |
|   > Creating network...        done  |
|   > Starting containers...          |
|                                      |
+--------------------------------------+
```

- Connects to the existing WebSocket for task output streaming
- Shows real-time docker-compose output (reuse existing task output component from `/tasks/[slug]`)
- Progress indication (spinner or progress bar)

#### State: `ready`

```
+--------------------------------------+
|                                      |
|      App is ready!                   |
|                                      |
|      Redirecting to                  |
|      myapp.example.com ...           |
|                                      |
+--------------------------------------+
```

- Brief pause (2-3 seconds for Traefik to update routing)
- Then `window.location.href = returnUrl`
- Fallback: if redirect fails, show a manual "Go to app" link

#### Error handling

- **App not found:** Show "App not found" with link to Scotty dashboard
- **No permission:** Show "You don't have permission to start this app"
- **Start failed:** Show error message from task output, with "Retry" button
- **Already running:** Skip to `ready` state, redirect immediately

### 5. Scotty Base URL Configuration

**New setting:** `api.base_url`

```yaml
# config/default.yaml
api:
  base_url: "https://scotty.example.com"
```

Used for:
- Constructing the redirect URL from app domain to landing page
- `is_scotty_domain()` check to distinguish Scotty requests from app requests

### 6. Return URL Validation (Security)

Prevent open redirect vulnerabilities. When redirecting back to `return_url`:

- Validate that the domain in `return_url` actually belongs to the app being started
- Reject arbitrary URLs

```rust
fn is_valid_return_url(return_url: &str, app: &AppData) -> bool {
    if let Ok(url) = Url::parse(return_url) {
        if let Some(host) = url.host_str() {
            return app_owns_domain(app, host);
        }
    }
    false
}
```

This validation should happen both:
- In the backend redirect handler (when constructing the initial redirect)
- In the frontend (before performing the final redirect to return_url)

## Implementation Plan

### Phase 1: Backend

1. **Add `find_app_by_domain()` to `SharedAppList`**
   - File: `scotty-core/src/apps/shared_app_list.rs`

2. **Add `api.base_url` setting**
   - File: `scotty-core/src/settings/api_server.rs`
   - File: `config/default.yaml`

3. **Replace fallback handler with Host-aware redirect**
   - File: `scotty/src/api/router.rs`
   - 302 redirect to `/landing/{app}?return_url=...`

### Phase 2: Frontend

4. **Create `/landing/[slug]` route with three-state UI**
   - `frontend/src/routes/landing/[slug]/+page.svelte` -- main component
   - `frontend/src/routes/landing/[slug]/+page.ts` -- load function
   - States: idle -> starting -> ready
   - "Start" button triggers auth-if-needed then start
   - `autostart=true` param for post-OAuth auto-trigger
   - Reuse existing task output WebSocket streaming
   - Auto-redirect to `return_url` on completion

### Phase 3: Traefik

5. **Document catch-all setup**
   - Example configs for container and file-provider approaches

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `scotty-core/src/apps/shared_app_list.rs` | Modify | Add `find_app_by_domain()` |
| `scotty-core/src/settings/api_server.rs` | Modify | Add `base_url` field |
| `config/default.yaml` | Modify | Add `api.base_url` |
| `scotty/src/api/router.rs` | Modify | Host-aware fallback with redirect |
| `frontend/src/routes/landing/[slug]/+page.svelte` | Create | Landing page UI |
| `frontend/src/routes/landing/[slug]/+page.ts` | Create | Page load function |

## Resolved Design Decisions

1. **Landing page layout:** Standalone minimal page (no dashboard shell). Keeps the UI focused and works for unauthenticated users.

2. **Task output detail level:** Full docker-compose log output via existing WebSocket task streaming.

3. **Unknown domain behavior:** Returns 404 with "No application is configured for this domain" HTML page.

4. **Deep path preservation:** Yes — the full path and query string are preserved in `return_url`.
