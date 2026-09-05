# kenkeep Index: apps / logs

↑ Parent: [apps](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
_None yet._

## Components (what exists)
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](map-follow-mode-unavailable-for-stopped-containers.md) to learn about: Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError. #logs #docker #websocket #frontend
- Open [**Log streaming behavior for stopped vs missing containers**](map-container-log-streaming-for-stopped-containers.md) to learn about: Stopped/exited containers return retained historical logs instead of an error; only a truly missing container is an error. #logs #docker #websocket

## By topic

### #docker
- Open [**Log streaming behavior for stopped vs missing containers**](map-container-log-streaming-for-stopped-containers.md) — Stopped/exited containers return retained historical logs instead of an error; only a truly missing container is an error.
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](map-follow-mode-unavailable-for-stopped-containers.md) — Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError.
- Open [**Each app gets its own dedicated Traefik proxy network**](../../traefik/map-traefik-per-app-proxy-network.md) — Scotty creates a per-app network (<network>--<app-name>) instead of one shared network, to avoid Docker DNS alias collisions.
### #logs
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](map-follow-mode-unavailable-for-stopped-containers.md) — Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError.
- Open [**Log streaming behavior for stopped vs missing containers**](map-container-log-streaming-for-stopped-containers.md) — Stopped/exited containers return retained historical logs instead of an error; only a truly missing container is an error.
### #websocket
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](map-follow-mode-unavailable-for-stopped-containers.md) — Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError.
- Open [**Log streaming behavior for stopped vs missing containers**](map-container-log-streaming-for-stopped-containers.md) — Stopped/exited containers return retained historical logs instead of an error; only a truly missing container is an error.
### #frontend
- Open [**Root layout loads user permissions when the user is logged in**](../../frontend/map-root-layout-loads-user-permissions-when-the-user-is-logged-in.md) — frontend/src/routes/+layout.svelte reactively calls loadUserPermissions() on isLoggedIn; permission-gated UI derives from permissionsLoaded.
- Open [**Frontend gates app actions per scope and shows refused dispatches inline**](../../frontend/map-frontend-per-app-permissions-and-action-errors.md) — hasPermission(appName, perm) resolves the app's scopes from appsStore (settings.scopes, default \['default'\]) and grants only from matching user scopes; unknown apps and \`_global\` keep any-scope semantics. Failed action dispatches surface the server's \`{error, message}\` text inline (alert on the detail page, error tooltip on the list button) instead of dying silently.
- Open [**Frontend src/ layout and dev-server proxy targets**](../../frontend/map-frontend-src-layout.md) — Frontend src/ splits into routes, stores (webSocketStore.ts, userStore.ts), generated (ts-rs output), and lib; dev server proxies /api and /ws to the backend.