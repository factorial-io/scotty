# Design: stopped-app-url-display

## Context

`frontend/src/components/app-service-button.svelte` is the single component rendering service URLs; it is used from the dashboard app list, the app detail page, and the service detail page. Today it branches on `service.status !== 'Running'` and renders a disabled service-name button instead of the domain links. Domains come from app settings via `ContainerState.domains` (scotty-core), so they are populated even when no container exists — the data needed to show URLs for stopped services is already in the API payload.

Scotty's Traefik default backend / landing page means a stopped app's URL is functional: visiting it shows a page offering to start the app. So links for stopped services should stay clickable.

## Goals / Non-Goals

**Goals:**
- Show domain links for non-running services identically to running ones, in a muted color.
- Keep the change confined to the one Svelte component.

**Non-Goals:**
- No backend, API-type, or ts-rs changes.
- No change to the landing page or Traefik behavior.
- No new per-status color scheme (only two visual states: running vs. not running).

## Decisions

1. **Branch on `domains.length`, not on status, for the fallback.** The component's top-level condition becomes "has domains" vs. "no domains". Status only selects the link style. This preserves the existing fallback (disabled service-name button) for services that never had domains, and is the minimal restructuring of the existing template.

2. **Style via a conditional class, keeping daisyUI conventions.** Running: current `btn btn-xs` styling unchanged. Not running: same button link with a muted treatment, e.g. `btn-ghost opacity-60` (or an equivalent dimmed daisyUI variant), plus a `title`/tooltip note that the service is stopped. daisyUI is already the styling system (`btn-ghost`, `btn-outline`, `opacity-*` are used elsewhere in the frontend), so no new CSS files or Tailwind config are needed. The exact class combo is an implementation detail — the requirement is only "visually distinct and dimmed in both light and dark themes".

3. **Links stay clickable for non-running services.** Alternative considered: render dimmed but non-clickable text. Rejected because the landing page turns the URL into a "start this app" affordance, and copyable/clickable URLs are the point of the change.

## Risks / Trade-offs

- [Users may expect a dimmed link to be disabled] → tooltip (`title`) on the dimmed link states the service is not running; the landing page also explains the state after click.
- [Status strings other than `Running` vary (`Exited`, `Empty`, `Created`, …)] → the logic treats everything ≠ `Running` uniformly, matching the existing check, so no status enumeration is needed.

## Migration Plan

Frontend-only; ships with the next build. Rollback = revert the component change.

## Open Questions

None.
