---
type: map
title: Default-backend landing page security properties
description: >-
  The landing-page redirect flow validates return_url against the app's own
  domain and still enforces normal manage permission to start the app.
tags:
  - security
  - landing-page
  - authorization
kk_schema_version: 3
kk_id: map-default-backend-security-model
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
The `return_url` used to send users back after starting an app is validated on the frontend before the final redirect: it must point to a domain that the app being started actually owns, otherwise it is rejected and the user is offered a link to the dashboard instead (open-redirect protection). The full path and query string of the original request are preserved in `return_url` so the user lands back on the exact page requested; as a trade-off, sensitive query parameters in the original URL will appear in the redirect `Location` header and may be logged by intermediate proxies. Starting an app from the landing page still goes through normal authorization: the user needs `manage` permission on the app's scope, or the start is rejected.
