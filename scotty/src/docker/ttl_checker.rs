use tracing::{info, instrument};

use crate::{app_state::SharedAppState, docker::stop_app::force_stop_app};
use scotty_core::apps::app_data::{AppData, AppTtl};
use scotty_core::utils::format::format_chrono_duration;

#[instrument(skip(app_state))]
pub async fn check_app_ttl(app_state: SharedAppState, app: &AppData) -> anyhow::Result<()> {
    let app_ttl = app.get_ttl();
    if app_ttl == AppTtl::Forever {
        info!("Ignoring {} as it is allowed to live forever", &app.name);
        return Ok(());
    }
    let app_ttl_seconds: u32 = app_ttl.into();
    let now = chrono::Local::now();
    let mut needs_termination = false;
    let mut remaining_ttl = 0;
    for service in &app.services {
        if !service.is_running() {
            continue;
        }
        if let Some(started_at) = service.started_at {
            let service_start_time = started_at;
            let service_ttl = now - service_start_time;
            if service_ttl.num_seconds() > app_ttl_seconds.into() {
                needs_termination = true;
            } else {
                remaining_ttl = std::cmp::max(
                    app_ttl_seconds - service_ttl.num_seconds() as u32,
                    remaining_ttl,
                );
            }
            info!(
                "{}.{} is running for {}; {} remaining before being stopped",
                app.name,
                service.service,
                format_chrono_duration(&service_ttl),
                format_chrono_duration(&chrono::Duration::seconds(remaining_ttl as i64))
            );
        }
    }
    if needs_termination {
        info!(
            "Stopping app, because TTL {} reached for {}",
            app_ttl_seconds, app.name
        );
        let _ = force_stop_app(app_state, app).await?;
    }
    Ok(())
}
