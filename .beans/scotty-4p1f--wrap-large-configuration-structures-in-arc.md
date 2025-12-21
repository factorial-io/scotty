---
# scotty-4p1f
title: Wrap large configuration structures in Arc
status: todo
type: task
priority: critical
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T13:33:37Z
parent: scotty-lbxn
---

# Description  Settings struct contains large nested HashMaps (blueprints, registries) that get cloned unnecessarily. Wrapping them in Arc would reduce clone overhead.  # Design  Location: scotty-core/src/settings/apps.rs  Settings contains large, rarely modified structures that are cloned when Settings is cloned.  Proposed solution: ```rust #[derive(Clone)] pub struct Apps {     pub root_folder: String,     pub blueprints: Arc&lt;HashMap&lt;String, AppBlueprint&gt;&gt;,  // Large, rarely modified     // ... } ```  Impact: Reduce Settings clone overhead Effort: 1-2 hours
