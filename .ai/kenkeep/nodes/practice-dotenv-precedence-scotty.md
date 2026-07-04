---
type: practice
title: .env / .env.local precedence and usage convention
description: >-
  Scotty auto-loads .env and .env.local; env vars > .env.local > .env, with .env
  committable and .env.local gitignored.
tags:
  - configuration
  - env
  - local-development
kk_schema_version: 3
kk_id: practice-dotenv-precedence-scotty
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Scotty automatically loads `.env` and `.env.local` files on startup. Precedence (highest to lowest): actual environment variables, then `.env.local`, then `.env`.

Convention: use `.env` for shared development defaults that can be committed, and `.env.local` for personal overrides that must be gitignored. Never commit secrets to version control, and don't rely on `.env` files in production — use real environment variables there instead. Both files are optional; the server starts normally if neither exists.
