---
type: map
title: 'scottyctl app:cp moves files between workstation and app containers'
description: >-
  CLI subcommand to copy files in/out of a service container, supports
  stdin/stdout piping.
tags:
  - scottyctl
  - cli
  - docker
kk_schema_version: 3
kk_id: map-scottyctl-app-cp
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
`scottyctl app:cp` moves files between your workstation and a service container, e.g.:

```shell
# Copy a file out of a container
scottyctl app:cp my-app:web:/var/log/app.log ./app.log

# Pipe a database dump into a container
mysqldump mydb | scottyctl app:cp - my-app:db:/tmp/dump.sql
```
