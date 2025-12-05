use crate::app_state::SharedAppState;
use chrono::Local;
use tracing::debug;

/// Sample AppList metrics and record them
///
/// Called periodically by the scheduler to track application counts, status distribution,
/// service counts per app, and health check age.
pub async fn sample_app_list_metrics(app_state: SharedAppState) {
    let apps = app_state.apps.get_apps().await;
    let total_apps = apps.apps.len() as u64;

    let m = super::metrics();

    // Count apps by status
    let mut status_counts = std::collections::HashMap::new();
    for app in &apps.apps {
        let status = app.status.to_string().to_lowercase();
        *status_counts.entry(status).or_insert(0u64) += 1;

        // Record service count for this app
        let service_count = app.services.len() as f64;
        m.record_app_services_count(service_count);

        // Record health check age if available
        if let Some(last_checked) = app.last_checked {
            let now = Local::now();
            let age_seconds = (now - last_checked).num_seconds() as f64;
            m.record_app_last_check_age(age_seconds);
        }
    }

    debug!(
        "App metrics: total={}, by_status={:?}",
        total_apps, status_counts
    );

    // Record metrics
    m.record_apps_total(total_apps);

    // Apps by status with labels
    for (status, count) in status_counts {
        m.record_apps_by_status(&status, count);
    }
}
