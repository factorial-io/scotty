---
# scotty-ch2z
title: Reduce cloning overhead by wrapping AppData in Arc
status: todo
type: task
priority: critical
created_at: 2025-12-21T12:44:46Z
updated_at: 2025-12-21T13:33:37Z
parent: scotty-lbxn
---

# Description  AppData is cloned on every access from SharedAppList. Wrapping AppData in Arc would make cloning cheap (just reference count increment) instead of copying all nested structures.  # Design  Location: scotty-core/src/apps/shared_app_list.rs:56-58  Current code clones entire AppData structure on every get_app() call. AppData contains multiple nested structures (containers, services, settings) making clones expensive.  Proposed solution: ```rust pub type SharedAppData = Arc&lt;AppData&gt;;  pub async fn get_app(&amp;self, app_name: &amp;str) -&gt; Option&lt;SharedAppData&gt; {     let t = self.apps.read().await;     t.get(app_name).map(Arc::clone)  // Only clones Arc, not data } ```  Impact: Major performance improvement for app data access paths Effort: 2-4 hours
