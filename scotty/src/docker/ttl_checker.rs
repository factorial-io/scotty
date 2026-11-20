use tracing::{info, instrument};

use crate::api::error::AppError;
use crate::docker::destroy_app::destroy_app;
use crate::{app_state::SharedAppState, docker::stop_app::force_stop_app};
use scotty_core::apps::app_data::{AppData, AppStatus, AppTtl};
use scotty_core::utils::format::format_duration;

#[instrument(skip(app_state))]
pub async fn check_app_ttl(app_state: SharedAppState, app: &AppData) -> anyhow::Result<()> {
    if let Some(running) = app_state.app_locks.current(&app.name) {
        info!(
            "Skipping TTL check of {}: {} is running; checking again on the next run",
            app.name, running.operation
        );
        return Ok(());
    }
    let app_ttl = app.get_ttl();
    if app_ttl == AppTtl::Forever {
        info!("Ignoring {} as it is allowed to live forever", &app.name);
        return Ok(());
    }
    let app_ttl_seconds: u32 = app_ttl.into();
    let now = jiff::Timestamp::now();
    let mut needs_termination = false;
    let mut remaining_ttl = 0;
    for service in &app.services {
        if !service.is_running() {
            continue;
        }
        if let Some(started_at) = service.started_at {
            let service_start_time = started_at;
            let service_ttl = now.duration_since(service_start_time);
            if service_ttl.as_secs() > app_ttl_seconds.into() {
                needs_termination = true;
            } else {
                remaining_ttl = std::cmp::max(
                    app_ttl_seconds - service_ttl.as_secs() as u32,
                    remaining_ttl,
                );
            }
            info!(
                "{}.{} is running for {}; {} remaining before being stopped",
                app.name,
                service.service,
                format_duration(&service_ttl),
                format_duration(&jiff::SignedDuration::from_secs(remaining_ttl as i64))
            );
        }
    }
    if needs_termination {
        let destroy = app.status != AppStatus::Unsupported
            && app.settings.as_ref().is_some_and(|s| s.destroy_on_ttl);
        info!(
            "{} app, because TTL {} reached for {}",
            if destroy { "Destroying" } else { "Stopping" },
            app_ttl_seconds,
            app.name
        );
        terminate(app_state, app, destroy).await?;
    }
    Ok(())
}

/// Stop or destroy an expired app. An operation that took the app since the
/// check above makes it busy: skipped, like a busy app at the check.
async fn terminate(app_state: SharedAppState, app: &AppData, destroy: bool) -> anyhow::Result<()> {
    let started = match destroy {
        false => force_stop_app(app_state, app).await,
        true => destroy_app(app_state, app, "ttl-destroy").await,
    };
    match started {
        Ok(_) => Ok(()),
        Err(e) => match e.downcast_ref::<AppError>() {
            Some(AppError::OperationInProgress { operation, .. }) => {
                info!(
                    "Skipping TTL check of {}: {} is running; checking again on the next run",
                    app.name, operation.operation
                );
                Ok(())
            }
            _ => Err(e),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scotty_core::apps::app_data::{ContainerState, ContainerStatus};

    /// A running app whose TTL ran out an hour ago; its compose file does
    /// not exist, so a stop task fails fast without docker.
    fn expired_app() -> AppData {
        AppData {
            name: "expired".into(),
            docker_compose_path: "/nonexistent/scotty-ttl/docker-compose.yml".into(),
            root_directory: "/nonexistent/scotty-ttl".into(),
            settings: Some(scotty_core::apps::app_data::AppSettings {
                time_to_live: AppTtl::Hours(1),
                ..Default::default()
            }),
            services: vec![ContainerState {
                service: "web".into(),
                status: ContainerStatus::Running,
                started_at: Some(jiff::Timestamp::now() - jiff::SignedDuration::from_hours(2)),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn busy_app_is_skipped_and_stopped_once_free() {
        let state = crate::api::test_utils::create_test_app_state_with_config(
            "tests/test_bearer_auth",
            None,
        )
        .await;
        let app = expired_app();

        let rebuild = state.app_locks.lock("expired", "rebuild").unwrap();
        check_app_ttl(state.clone(), &app).await.unwrap();
        assert!(
            state.task_manager.get_task_list().await.is_empty(),
            "a busy app is not stopped"
        );
        drop(rebuild);

        check_app_ttl(state.clone(), &app).await.unwrap();
        let tasks = state.task_manager.get_task_list().await;
        assert_eq!(tasks.len(), 1, "stopped on the next check");
    }

    /// The app is taken between the check and the stop: skipped, not an
    /// error.
    #[tokio::test]
    async fn app_taken_after_the_check_is_skipped() {
        let state = crate::api::test_utils::create_test_app_state_with_config(
            "tests/test_bearer_auth",
            None,
        )
        .await;
        let app = expired_app();
        let _rebuild = state.app_locks.lock("expired", "rebuild").unwrap();
        for destroy in [false, true] {
            terminate(state.clone(), &app, destroy).await.unwrap();
        }
        assert!(state.task_manager.get_task_list().await.is_empty());
    }
}
