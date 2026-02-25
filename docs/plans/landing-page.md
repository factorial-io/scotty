# Scotty Landing Page for Stopped Apps

## Problem

When a user visits an app URL (e.g., `myapp.example.com`) and the app is not running, Traefik has no backend to route to -- the request fails with a gateway error. There is no way for the user to start the app without going to the Scotty dashboard first.

## Goal

Provide a seamless experience: when a stopped app's domain is visited, the user is redirected to a landing page on Scotty's domain where they can log in, start the app, and be redirected back to the original URL once the app is running.

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
Scotty issues HTTP redirect (302) to:
  https://scotty.example.com/landing/myapp?return_url=https://myapp.example.com
        |
        v
Landing page is now on Scotty's domain -- full auth available
        |
        v
User logs in (standard Scotty OAuth, no cross-domain issues)
        |
        v
User clicks "Start App"
        |
        v
App starts (docker-compose up), landing page shows progress
        |
        v
Once app is ready, redirect back to return_url (myapp.example.com)
        |
        v
Traefik now routes to the running app -> user sees the real app
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
            for service in &settings.public_services {
                // Check custom domains
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

        // Check container-level domains (from previously-running state)
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

### 3. Host Header Detection and Redirect

**What:** When Scotty receives a request via the Traefik catch-all (Host header doesn't match Scotty's own domain), look up the app and issue a redirect to Scotty's landing page.

**Where:** Replace the fallback handler in `scotty/src/api/router.rs`

Currently, `router.rs` line 441 has:
```rust
router.fallback(|uri: axum::http::Uri| async move { serve_embedded_file(uri).await })
```

Replace with a Host-aware fallback:

```rust
async fn landing_or_frontend_handler(
    Host(hostname): Host,
    State(state): State<SharedAppState>,
    uri: axum::http::Uri,
) -> Response<Body> {
    // 1. Check if this is a request for Scotty's own domain
    if is_scotty_domain(&state, &hostname) {
        return serve_embedded_file(uri).await;
    }

    // 2. Check if this domain belongs to a known app
    if let Some(app) = state.app_list.find_app_by_domain(&hostname).await {
        // Build the return URL from the original request
        let scheme = if state.settings.traefik.use_tls { "https" } else { "http" };
        let return_url = format!("{}://{}{}", scheme, hostname, uri.path());
        let scotty_base = &state.settings.api.base_url; // e.g., "https://scotty.example.com"

        let redirect_url = format!(
            "{}/landing/{}?return_url={}",
            scotty_base,
            urlencoding::encode(&app.name),
            urlencoding::encode(&return_url),
        );

        return Response::builder()
            .status(StatusCode::FOUND) // 302
            .header("Location", redirect_url)
            .body(Body::empty())
            .unwrap();
    }

    // 3. Unknown domain -- serve a generic "unknown app" page or 404
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("content-type", "text/html")
        .body(Body::from("<h1>App not found</h1><p>No app is configured for this domain.</p>"))
        .unwrap()
}
```

**Key advantage of the redirect approach:**
- No cross-domain auth issues -- everything happens on Scotty's domain
- No self-contained HTML page needed -- reuse the SvelteKit frontend
- No new public API endpoints needed -- the frontend uses existing authenticated APIs
- OAuth, WebSocket, localStorage all work natively

### 4. Landing Page (SvelteKit Route)

**What:** A new route in the existing SvelteKit frontend at `/landing/[app_name]`.

**Why SvelteKit (not self-contained HTML):**
- Since the redirect brings the user to Scotty's domain, the full SvelteKit app is available
- Reuses existing auth infrastructure (OAuth flow, token management)
- Reuses existing API client and WebSocket handling
- Consistent styling with the Scotty dashboard (DaisyUI/Tailwind)

**Where:** `frontend/src/routes/landing/[slug]/+page.svelte`

**The page shows:**

1. **App info section:**
   - App name
   - Current status (Stopped / Starting / Running)
   - The domain the user originally visited

2. **Auth section (if not logged in):**
   - "Login to start this app" with the standard Scotty login flow
   - After login, return to this same landing page

3. **Action section (if logged in):**
   - "Start App" button
   - Progress indicator showing docker-compose output (via existing WebSocket task streaming)

4. **Auto-redirect (after app starts):**
   - Once the task completes successfully, redirect to the `return_url` query parameter
   - Show a brief "App is ready, redirecting..." message
   - The `return_url` is the original app URL the user visited

**Page logic (pseudocode):**

```svelte
<script>
  import { page } from '$app/stores';

  const appName = $page.params.slug;
  const returnUrl = $page.url.searchParams.get('return_url');

  // Use existing API to get app info
  // GET /api/v1/authenticated/apps/info/{appName}

  // Start app button calls:
  // GET /api/v1/authenticated/apps/run/{appName}

  // Monitor task via existing WebSocket
  // When task completes -> redirect to returnUrl
</script>
```

### 5. Scotty Base URL Configuration

**New setting:** `api.base_url` -- Scotty's public-facing base URL, used to construct the redirect target.

```yaml
# config/default.yaml
api:
  base_url: "https://scotty.example.com"
```

**Detection logic for `is_scotty_domain()`:**

```rust
fn is_scotty_domain(state: &SharedAppState, hostname: &str) -> bool {
    // Derive from base_url
    if let Ok(url) = Url::parse(&state.settings.api.base_url) {
        if let Some(host) = url.host_str() {
            return hostname == host;
        }
    }

    // Fallback: derive from oauth frontend_base_url
    if let Ok(url) = Url::parse(&state.settings.api.oauth.frontend_base_url) {
        if let Some(host) = url.host_str() {
            return hostname == host;
        }
    }

    // If nothing configured, don't redirect (assume it's Scotty)
    true
}
```

### 6. Redirect After App Start

**What:** Once the app is running, redirect back to the original URL.

**How:** The landing page SvelteKit component:

1. Starts the app via existing `GET /api/v1/authenticated/apps/run/{app_id}`
2. Monitors the task via existing WebSocket connection (task output streaming)
3. When the task status becomes "Finished":
   - Show "App is ready! Redirecting..." message
   - Wait 2-3 seconds (for Traefik to pick up the new container labels)
   - `window.location.href = returnUrl`
4. The user lands on `myapp.example.com` -- Traefik now routes to the running app

**Edge case -- return_url validation:**
- Only allow redirects to domains that actually belong to the app being started
- Prevent open redirect vulnerabilities by validating `return_url` against the app's configured domains

```rust
fn is_valid_return_url(return_url: &str, app: &AppData) -> bool {
    if let Ok(url) = Url::parse(return_url) {
        if let Some(host) = url.host_str() {
            // Check if the return URL's domain matches one of the app's domains
            return app_owns_domain(app, host);
        }
    }
    false
}
```

## Implementation Plan

### Phase 1: Backend -- Redirect Logic

1. **Add `find_app_by_domain()` to `SharedAppList`**
   - File: `scotty-core/src/apps/shared_app_list.rs`
   - Search through all apps' settings and container states for matching domains

2. **Add `api.base_url` setting**
   - File: `scotty-core/src/settings/api_server.rs`
   - Scotty's own public-facing base URL for constructing redirects

3. **Replace fallback handler with Host-aware redirect handler**
   - File: `scotty/src/api/router.rs`
   - Inspect Host header, look up app, issue 302 redirect to `/landing/{app}`

### Phase 2: Frontend -- Landing Page Route

4. **Create `/landing/[slug]` SvelteKit route**
   - File: `frontend/src/routes/landing/[slug]/+page.svelte`
   - Shows app info, login prompt, start button
   - Uses existing auth, API client, and WebSocket infrastructure
   - Reads `return_url` from query parameters

5. **Add redirect-after-start logic**
   - Monitor task completion via WebSocket
   - Redirect to `return_url` once app is running
   - Validate `return_url` against app's domains (prevent open redirect)

### Phase 3: Traefik Configuration

6. **Document Traefik catch-all setup**
   - Provide example configurations for container and file-provider approaches
   - Add to deployment docs

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `scotty-core/src/apps/shared_app_list.rs` | Modify | Add `find_app_by_domain()` method |
| `scotty-core/src/settings/api_server.rs` | Modify | Add `base_url` field |
| `scotty/src/api/router.rs` | Modify | Replace fallback with Host-aware redirect handler |
| `config/default.yaml` | Modify | Add `api.base_url` setting |
| `frontend/src/routes/landing/[slug]/+page.svelte` | Create | Landing page UI |
| `frontend/src/routes/landing/[slug]/+page.ts` | Create | Page load function (fetch app info) |

## What This Approach Eliminates

Compared to the previous plan (serving a landing page on the app's domain):

- ~~Self-contained HTML page~~ -- uses SvelteKit route instead
- ~~Cross-domain auth problem~~ -- everything on Scotty's domain
- ~~Custom `X-Scotty-Landing` marker headers~~ -- not needed
- ~~Public `/api/v1/landing/info` endpoint~~ -- not needed
- ~~OAuth wildcard redirect URI configuration~~ -- not needed
- ~~`landing_page.rs` module~~ -- not needed

## Open Questions / Decisions Needed

1. **Landing page styling:** Should it be a minimal focused page, or integrate into the existing dashboard layout?

2. **Domain index performance:** For large deployments, should we build an in-memory domain-to-app index, or is O(n) scan acceptable?

3. **Running app edge case:** If the app is "Running" but Traefik still sent us the request (race condition during startup), should we redirect to the landing page or wait briefly and redirect to the app URL directly?

4. **Unknown domain behavior:** When the domain doesn't match any app, should we show a 404, redirect to the Scotty dashboard, or show a generic page?

5. **`return_url` for non-web paths:** If the user visited `myapp.example.com/some/deep/path`, should that full path be preserved in the return URL?
