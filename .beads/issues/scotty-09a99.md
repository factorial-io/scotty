---
title: Wrap large configuration structures in Arc
status: open
priority: 1
issue_type: task
labels:
- performance
- refactoring
created_at: 2025-10-26T20:21:10.606337+00:00
updated_at: 2025-11-24T20:17:25.587875+00:00
---

# Description

Settings struct contains large nested HashMaps (blueprints, registries) that get cloned unnecessarily. Wrapping them in Arc would reduce clone overhead.

# Design

Location: scotty-core/src/settings/apps.rs

Settings contains large, rarely modified structures that are cloned when Settings is cloned.

Proposed solution:
```rust
#[derive(Clone)]
pub struct Apps {
    pub root_folder: String,
    pub blueprints: Arc&lt;HashMap&lt;String, AppBlueprint&gt;&gt;,  // Large, rarely modified
    // ...
}
```

Impact: Reduce Settings clone overhead
Effort: 1-2 hours
