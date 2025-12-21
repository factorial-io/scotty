---
# scotty-8tbf
title: Add AppList metrics for application monitoring
status: completed
type: task
priority: high
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T12:44:45Z
---

# Description  Instrument SharedAppList to track number of apps, their states (stopped/starting/running/etc), services per app, and health check age.  # Design  Add AppList metrics to ScottyMetrics struct: - apps_total (Gauge) - Total number of managed apps - apps_by_status (Gauge) - Apps grouped by status labels:   * status="stopped"   * status="starting"   * status="running"   * status="creating"   * status="destroying"   * status="unsupported" - app_services_count (Histogram) - Distribution of services per app - app_last_check_age_seconds (Histogram) - Time since last health check  Implementation approach: 1. Add metrics to scotty/src/metrics/instruments.rs 2. Instrument scotty-core/src/apps/shared_app_list.rs:    - add_app() / remove_app(): update apps_total    - Background sampler task (30-60s interval):      * Iterate apps HashMap      * Count by status      * Sample services.len() distribution      * Calculate last_checked age 3. Spawn sampler task in scotty/src/main.rs or via AppState  Data sources: - SharedAppList.apps HashMap size = total apps - AppData.status: Stopped | Starting | Running | Creating | Destroying | Unsupported - AppData.services.len() = service count per app - AppData.last_checked timestamp for age calculation  Note: Requires access to SharedAppList from metrics, may need to pass via background task or add to AppState.  # Acceptance Criteria  - App metrics exported to OTLP - Metrics accurately reflect app states - Status distribution correct - Service count distribution tracked - Health check age visible - Dashboard panels created for app monitoring - Minimal performance overhead from sampling
