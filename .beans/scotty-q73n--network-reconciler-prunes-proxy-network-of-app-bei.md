---
# scotty-q73n
title: 'Network reconciler prunes proxy network of app being created (GH #893)'
status: in-progress
type: bug
priority: normal
created_at: 2026-09-04T20:19:34Z
updated_at: 2026-09-04T20:25:12Z
---

The scheduled reconciler's app snapshot comes from a find_apps() scan taken at tick start. An app created after the scan but before prune_orphans() has its freshly created proxy network classified as orphan and removed, so compose up fails with 'network declared as external, but could not be found'. Fix: re-check the app directory on disk before pruning.

- [x] Guard prune_orphans with a fresh app-directory existence check
- [x] Unit test for the guard (unit test for owning_app_dir_exists plus Docker-backed regression test prune_keeps_network_of_app_created_after_snapshot, verified to fail without the guard)
- [x] Reply on GitHub issue 893

## Summary of Changes

Corrected the issue analysis: the reconciler classifies against the find_apps() filesystem snapshot taken at tick start, not the shared app list, so a create landing between the scan and prune_orphans() is misclassified as orphan. Fix in scotty/src/docker/loadbalancer/network_reconciler.rs: prune_orphans() re-checks <root_folder>/<slug(app)> on disk right before removal (owning_app_dir_exists). Ownership, not liveness, stays the prune criterion, matching how stopped apps already keep their networks.
