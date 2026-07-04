---
type: map
title: 'app:create injects a noindex X-Robots-Tag by default'
description: >-
  Scotty adds X-Robots-Tag: none,noarchive,... to every app response unless
  --allow-robots is set.
tags:
  - cli
  - app-create
  - traefik
  - seo
kk_schema_version: 3
kk_id: map-cli-app-create-robots-header-default
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
By default, `app:create` causes Scotty to inject an `X-Robots-Tag: none, noarchive, nosnippet, notranslate, noimageindex` header into all of an app's responses, preventing search engines from indexing it (also suppressing caching, snippets, translation, and image indexing).

The `--allow-robots` flag disables this behavior. It is not supported by all proxies.
