---
type: practice
title: Test placement and tooling conventions
description: >-
  Unit tests are colocated with implementation; integration tests live in
  scotty/tests; axum-test and wiremock are used for HTTP/mocking.
tags:
  - testing
  - conventions
kk_schema_version: 3
kk_id: practice-testing-conventions
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Unit tests are colocated with implementation. Integration tests live in `scotty/tests/`. The project uses `axum-test` for HTTP testing and `wiremock` for mocking external services.
