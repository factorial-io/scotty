---
type: practice
title: OAuth config has two distinct URLs that must not be confused
description: >-
  redirect_url is the backend's OAuth callback (must match the OIDC provider's
  app config); frontend_base_url is the frontend's base URL Scotty redirects
  users back to.
tags:
  - oauth
  - configuration
  - gotcha
kk_schema_version: 3
kk_id: practice-oauth-redirect-url-vs-frontend-base-url
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
Scotty's OAuth config has two separate URL settings that are easy to conflate: `redirect_url` is the backend's `/api/oauth/callback` endpoint, which must exactly match the redirect URI registered in the OIDC provider's OAuth application. `frontend_base_url` is the base URL (no path) of the frontend application, used by Scotty to redirect users back after OAuth completes (Scotty appends `/oauth/callback?session_id=xyz`).

Both must match the actual production domain — using `localhost` for either in production will break the OAuth flow.
