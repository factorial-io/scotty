---
type: practice
title: apps.root_folder must match the host mount path when Scotty runs in Docker
description: >-
  If Scotty runs containerized, the apps root_folder path inside the container
  must equal the host path, or docker-compose fails to run apps.
tags:
  - docker
  - configuration
  - gotcha
kk_schema_version: 3
kk_id: practice-root-folder-must-match-docker-mount-path
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
`apps.root_folder` is the folder where apps are stored (default `./apps`). If you run Scotty in a Docker container and mount the apps folder into it, the path inside the container and the path on the host must be the same. Otherwise docker-compose fails to run the apps, because there's a mismatch between the local path Scotty sees and the path on the host where the Docker daemon actually runs.
