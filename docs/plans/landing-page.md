# Scotty Landing Page for Stopped Apps

## Problem

When a user visits an app URL (e.g., `myapp.example.com`) and the app is not running, Traefik has no backend to route to -- the request fails with a gateway error. There is no way for the user to start the app without going to the Scotty dashboard first.

## Goal

Provide a seamless experience: when a stopped app's domain is visited, the user sees a landing page offering to log in and start the app. Once started, the user is automatically redirected to the original URL.

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
Scotty serves a self-contained landing page
        |
        v
Landing page shows: app name, status, login + start button
        |
        v
User logs in (OAuth) and clicks "Start"
        |
        v
App starts (docker-compose up -d)
        |
        v
Landing page polls until app is ready
        |
        v
Traefik picks up new container labels, routes to real app
        |
        v
User is redirected to original URL -> now sees the running app
```

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

**Key detail:** The priority `1` ensures this is the last resort. Running app containers will have auto-generated Traefik routers with default priority (based on rule length), which is always higher than 1.

### 2. Domain-to-App Lookup

**What:** Add a method to resolve which app owns a given domain.

**Where:** `scotty-core/src/apps/shared_app_list.rs`

**New method on `SharedAppList`:**

```rust
/// Look up an app by one of its domains.
/// Searches through all apps' settings (configured domains) and
/// container states (runtime domains).
pub async fn find_app_by_domain(&self, domain: &str) -> Option<AppData> {
    let apps = self.apps.read().await;
    for app in apps.values() {
        // Check settings-based domains
        if let Some(settings) = &app.settings {
            // Check base domain (e.g., "myapp.example.com")
            // The convention is "{service}.{settings.domain}" for services
            // without custom domains
            for service in &settings.public_services {
                if service.domains.contains(&domain.to_string()) {
                    return Some(app.clone());
                }
                // Check auto-generated domain: {service}.{settings.domain}
                let auto_domain = format!("{}.{}", service.service, settings.domain);
                if auto_domain == domain {
                    return Some(app.clone());
                }
            }
        }

        // Check container-level domains (from running/previously-running state)
        for container in &app.services {
            if container.domains.contains(&domain.to_string()) {
                return Some(app.clone());
            }
        }
    }
    None
}
```

**Performance note:** This is O(n) over all apps. For deployments with hundreds of apps, consider adding an in-memory domain index (`HashMap<String, String>` mapping domain -> app_name) that is rebuilt when the app list updates.

### 3. Host Header Detection in Scotty

**What:** When Scotty receives a request with a Host header that does not match its own domain, check if it belongs to a known app and serve the landing page.

**Where:** New handler/middleware in `scotty/src/api/`

**Approach -- Fallback handler replacement:**

Currently, `router.rs` line 441 has:
```rust
router.fallback(|uri: axum::http::Uri| async move { serve_embedded_file(uri).await })
```

Replace this with a smarter fallback that inspects the Host header:

```rust
router.fallback(landing_or_frontend_handler)
```

**The `landing_or_frontend_handler`:**

```rust
async fn landing_or_frontend_handler(
    Host(hostname): Host,
    State(state): State<SharedAppState>,
    uri: axum::http::Uri,
) -> Response<Body> {
    // 1. Check if this is a request for Scotty's own domain
    //    (compare against configured scotty domain or known API host)
    if is_scotty_domain(&state, &hostname) {
        return serve_embedded_file(uri).await;
    }

    // 2. Check if this domain belongs to a known app
    if let Some(app) = state.app_list.find_app_by_domain(&hostname).await {
        if app.status == AppStatus::Stopped || app.status == AppStatus::Starting {
            return serve_landing_page(&app, &hostname);
        }
        // App is running but we still got the request -- might be
        // a timing issue (Traefik hasn't picked up labels yet).
        // Redirect or show "starting..." page.
    }

    // 3. Unknown domain -- serve the regular frontend (or 404)
    serve_embedded_file(uri).await
}
```

**New setting needed:** Scotty needs to know its own domain to distinguish "request for Scotty UI" vs "request for a stopped app". Add to settings:

```yaml
# config/default.yaml
api:
  domain: "scotty.example.com"  # Scotty's own domain
```

Alternatively, use the existing `api.oauth.frontend_base_url` to derive this.

### 4. Landing Page (Self-Contained HTML)

**What:** A single, self-contained HTML page with inline CSS and JavaScript, embedded in the Scotty binary.

**Why not part of the SvelteKit frontend:**
- Served on a different origin (the app's domain, not Scotty's)
- Needs to be lightweight and independent
- No build step dependency for a single page
- Auth tokens/cookies are per-origin, so the SvelteKit app's auth state doesn't apply

**Where:** `scotty/src/landing_page.rs` (new file)

**The page includes:**

1. **App info section:**
   - App name
   - Current status (Stopped / Starting)
   - List of services

2. **Action section:**
   - "Login" button (if not authenticated)
   - "Start App" button (if authenticated)
   - Progress indicator during startup

3. **Auto-redirect logic:**
   - After starting, poll the app's actual URL
   - Once the real app responds (not the landing page), redirect

**Template approach:**

```rust
fn serve_landing_page(app: &AppData, hostname: &str) -> Response<Body> {
    let html = LANDING_PAGE_TEMPLATE
        .replace("{{APP_NAME}}", &app.name)
        .replace("{{APP_STATUS}}", &format!("{:?}", app.status))
        .replace("{{HOSTNAME}}", hostname);

    Response::builder()
        .header("content-type", "text/html")
        .body(Body::from(html))
        .unwrap()
}
```

**Landing page JavaScript logic:**

```javascript
// 1. Check for existing auth token
const token = localStorage.getItem('scotty_token');

// 2. If no token, show login button
//    Login redirects to OAuth flow on the SAME domain
//    (works because Traefik routes everything to Scotty)

// 3. If token exists, validate it
//    GET /api/v1/authenticated/validate-token

// 4. Start app button calls:
//    GET /api/v1/authenticated/apps/run/{app_id}
//    with Authorization: Bearer <token>

// 5. Poll for readiness:
//    - Connect to WebSocket for real-time task updates
//    - OR poll a lightweight status endpoint

// 6. Redirect when ready:
//    Once app containers are up, Traefik will route to them.
//    Detect this by fetching the current URL and checking if
//    the response is NOT the landing page (e.g., check for a
//    custom header or absence of a landing-page marker).
```

### 5. Landing Page API Endpoints

**New public endpoints** (no auth required):

```
GET /api/v1/landing/info
  - Reads Host header
  - Returns: { app_name, status, services[], needs_auth }
  - Public: yes (needed before login)
```

**Existing endpoints used by the landing page:**

```
POST /api/v1/login                              -- Login with credentials
GET  /oauth/authorize                           -- Start OAuth web flow
GET  /api/v1/authenticated/apps/run/{app_id}    -- Start the app (needs auth)
GET  /ws                                        -- WebSocket for task progress
```

### 6. Authentication on the Landing Page

**Challenge:** The landing page is served on the app's domain (e.g., `myapp.example.com`), not Scotty's domain. Auth tokens stored for Scotty's domain are not accessible.

**Solution:** Since Traefik routes all traffic for the stopped app's domain to Scotty, the full Scotty API (including OAuth) is available on the app's domain. The auth flow works natively:

1. Landing page on `myapp.example.com` initiates OAuth at `myapp.example.com/oauth/authorize`
2. Traefik routes this to Scotty
3. Scotty starts the OAuth flow, redirecting to the OAuth provider
4. The OAuth provider redirects back to the callback URL
5. **Key requirement:** The OAuth callback URL must be dynamically set to the current domain, OR the OAuth provider must accept wildcard/multiple redirect URIs
6. Token is stored in `localStorage` for `myapp.example.com`

**OAuth callback configuration options:**

- **Wildcard redirect URI:** Configure OAuth provider to accept `https://*.example.com/api/oauth/callback` (not all providers support this)
- **Dynamic redirect URI:** Scotty passes the current Host as the redirect URI to the OAuth provider. The provider must have this domain pre-registered or support dynamic URIs
- **Proxy through Scotty domain:** Login happens on `scotty.example.com`, then a one-time token is passed back to `myapp.example.com` via URL parameter
- **Bearer token input:** For simpler setups, allow the user to paste a bearer token obtained from `scottyctl auth:login`

**Recommended approach:** Support multiple strategies, configured per deployment:

1. **OAuth with wildcard redirect** (best UX, requires OAuth provider support)
2. **Redirect via Scotty domain** (works with any OAuth provider)
3. **Bearer token paste** (fallback, works without OAuth)
4. **Dev mode** (no auth needed, auto-start)

### 7. Redirect After App Start

**What:** Once the app is running, automatically redirect the user to the original URL so they see the actual app.

**How the redirect works:**

1. Landing page starts the app via API
2. Landing page monitors task progress (via WebSocket or polling)
3. Once the task reports "Finished" and containers are running:
   - Traefik detects the new container labels (near-instant with Docker provider)
   - Traefik updates its routing table
   - The app's domain now routes to the actual app containers
4. Landing page verifies by fetching the current URL:
   - If the response does NOT contain the landing page marker header
     (`X-Scotty-Landing: true`), the real app is responding
5. Landing page does `window.location.reload()` to show the real app

**Timing considerations:**

- Traefik's Docker provider detects label changes within seconds
- There may be a brief overlap where some requests still hit Scotty
- The landing page should add a small delay (2-3 seconds) after task completion before redirecting
- Use an exponential backoff retry on the redirect check

**Custom header for detection:**

```rust
// In serve_landing_page()
Response::builder()
    .header("content-type", "text/html")
    .header("X-Scotty-Landing", "true")  // Marker header
    .body(Body::from(html))
```

```javascript
// In landing page JS
async function checkAppReady() {
    try {
        const resp = await fetch(window.location.href, { redirect: 'follow' });
        if (!resp.headers.get('X-Scotty-Landing')) {
            // Real app is responding!
            window.location.reload();
            return true;
        }
    } catch (e) {
        // App not ready yet
    }
    return false;
}
```

### 8. Scotty Domain Configuration

**New setting:** `api.domain` -- Scotty's own domain, used to distinguish landing page requests from dashboard requests.

```yaml
# config/default.yaml
api:
  domain: ""  # Empty = treat all unknown hosts as potential app domains
```

**Detection logic:**

```rust
fn is_scotty_domain(state: &SharedAppState, hostname: &str) -> bool {
    // If api.domain is configured, use exact match
    if !state.settings.api.domain.is_empty() {
        return hostname == state.settings.api.domain;
    }

    // Fallback: derive from frontend_base_url
    if let Ok(url) = Url::parse(&state.settings.api.oauth.frontend_base_url) {
        if let Some(host) = url.host_str() {
            return hostname == host;
        }
    }

    // If nothing configured, check if hostname matches
    // any known app domain. If not, assume it's Scotty.
    false
}
```

## Implementation Plan

### Phase 1: Core Backend (Rust)

1. **Add `find_app_by_domain()` to `SharedAppList`**
   - File: `scotty-core/src/apps/shared_app_list.rs`
   - Search through all apps' settings and container states for matching domains

2. **Add `api.domain` setting**
   - File: `scotty-core/src/settings/api_server.rs`
   - New optional field for Scotty's own domain

3. **Create landing page HTML template**
   - File: `scotty/src/landing_page.rs`
   - Self-contained HTML with inline Tailwind CSS (via CDN or minimal inline styles)
   - Inline JavaScript for login, app start, and redirect logic

4. **Create landing page API endpoint**
   - File: `scotty/src/api/rest/handlers/landing.rs`
   - `GET /api/v1/landing/info` -- public endpoint returning app info for current Host

5. **Replace fallback handler with Host-aware handler**
   - File: `scotty/src/api/router.rs`
   - Smart fallback that checks Host header and serves landing page or SPA frontend

### Phase 2: Landing Page UI

6. **Design and build the landing page HTML**
   - App name + status display
   - Login button (linking to OAuth or showing token input)
   - Start button with loading state
   - Progress indicator (connecting to WebSocket for task output)
   - Auto-redirect logic with readiness check

### Phase 3: Auth Integration

7. **Support OAuth on app domains**
   - Modify OAuth flow to accept dynamic redirect URIs (or configure wildcard)
   - Ensure tokens work for API calls on the app's domain

8. **Fallback auth methods**
   - Bearer token paste input for simpler setups
   - Dev mode auto-authentication

### Phase 4: Traefik Configuration

9. **Document Traefik catch-all setup**
   - Provide example configurations for both container and file-provider approaches
   - Add to deployment docs

10. **Optional: Auto-configure catch-all**
    - Scotty could generate its own Traefik labels when running as a container

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `scotty-core/src/apps/shared_app_list.rs` | Modify | Add `find_app_by_domain()` method |
| `scotty-core/src/settings/api_server.rs` | Modify | Add `domain` field to `ApiServer` |
| `scotty/src/landing_page.rs` | Create | Landing page HTML template + serve function |
| `scotty/src/api/rest/handlers/landing.rs` | Create | Landing page API endpoint |
| `scotty/src/api/rest/handlers/mod.rs` | Modify | Register landing module |
| `scotty/src/api/router.rs` | Modify | Replace fallback with Host-aware handler |
| `scotty/src/lib.rs` or `scotty/src/main.rs` | Modify | Register new module |
| `config/default.yaml` | Modify | Add `api.domain` setting |

## Open Questions / Decisions Needed

1. **OAuth redirect URI strategy:** Which approach for handling OAuth on app domains? Wildcard redirect, proxy through Scotty domain, or bearer token fallback?

2. **Landing page styling:** Should it match Scotty's dashboard theme (DaisyUI) or be minimal/branded?

3. **Domain index performance:** For large deployments, should we build an in-memory domain-to-app index, or is O(n) scan acceptable?

4. **Partial match support:** Should `myapp.example.com` also match if the configured domain is just `example.com` with service prefix `web.example.com`?

5. **Running app edge case:** If the app is "Running" but Traefik still sent us the request (race condition), should we show a "Loading..." page or redirect immediately?

6. **Multi-service apps:** If an app has multiple services with different domains, should the landing page show all of them or just the one being accessed?
