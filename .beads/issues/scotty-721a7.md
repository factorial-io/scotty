---
title: Fix changelog and release process
status: in_progress
priority: 2
issue_type: task
created_at: 2025-11-28T14:51:35.718990+00:00
updated_at: 2025-11-28T14:51:40.803744+00:00
---

# Description

Design and implement automated changelog generation with cargo-release to ensure GitHub releases work reliably. Problem: git-cliff doesn't always create correct changelogs, commits listed for wrong versions, GitHub action fails when changelog section missing for a version.
