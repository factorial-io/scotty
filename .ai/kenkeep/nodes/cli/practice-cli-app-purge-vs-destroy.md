---
type: practice
title: 'app:purge only clears ephemeral data; app:destroy is the irreversible one'
description: 'app:purge keeps volumes/DBs, app:destroy removes everything including images.'
tags:
  - cli
  - app-purge
  - app-destroy
  - gotcha
kk_schema_version: 3
kk_id: practice-cli-app-purge-vs-destroy
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
`app:purge` removes temporary data of an app (logs, temporary docker containers, other ephemeral data) but explicitly does not delete persistent data like volumes or databases. If the app was running, purge stops it first.

`app:destroy` is the irreversible counterpart: it stops the app, removes all ephemeral and persistent data, removes the app from the Scotty server, and deletes images no longer used elsewhere. Use purge for cleanup without data loss, destroy only when the app should go away entirely.
