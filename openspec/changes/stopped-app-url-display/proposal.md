# Proposal: stopped-app-url-display

## Why

Stopped applications currently hide their URLs in the frontend: `app-service-button.svelte` renders a disabled button with only the service name whenever a service is not `Running`. But service domains are always known (they come from app settings, not from live containers), and Scotty's Traefik landing page makes a stopped app's URL genuinely useful — visiting it offers to start the app. Hiding the URL removes an easy copy/click affordance for no benefit.

## What Changes

- Services of stopped/non-running apps show their URLs as links, exactly like running services (same layout, same icon, same domains).
- Non-running service URLs are visually distinguished by a different color (muted/dimmed styling) so users can tell at a glance the service is not up.
- Links remain clickable (the landing page handles the stopped case by offering to start the app).
- A service with no domains keeps the current fallback (plain service-name button).
- No backend or API changes.

## Capabilities

### New Capabilities

- `service-url-visibility`: How the frontend displays service URLs depending on the service's container status (running vs. not running vs. no domains).

### Modified Capabilities

<!-- none — existing specs (container-logs, app-file-transfer) are unaffected -->

## Impact

- `frontend/src/components/app-service-button.svelte` — the only rendering site; used from the dashboard list, app detail, and service detail pages.
- No changes to Rust crates, API types, or Traefik/landing-page behavior.
