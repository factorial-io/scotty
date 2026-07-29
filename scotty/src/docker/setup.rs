use tracing::instrument;

use crate::{
    app_state::SharedAppState,
    docker::{find_apps::find_apps, loadbalancer::network_reconciler, ttl_checker::check_app_ttl},
};

pub async fn setup_docker_integration(
    app_state: SharedAppState,
) -> anyhow::Result<tokio::task::JoinHandle<anyhow::Result<()>>> {
    // Find all running apps on startup. This also reconciles the load
    // balancer's per-app proxy networks, so a host whose Traefik container was
    // recreated while scotty was down is repaired before serving any request.

    schedule_app_check(app_state.clone()).await;

    // Watch for the Traefik container starting, so a recreate is repaired within
    // seconds instead of at the next running-app check. Purely an accelerator:
    // the scheduled check below reconciles regardless.
    if app_state.settings.traefik.watch_docker_events {
        if crate::docker::loadbalancer::traefik_target(&app_state.settings).is_some() {
            tracing::info!("Reconciling proxy networks on docker events and on every app check");
            let app_state = app_state.clone();
            crate::metrics::spawn_instrumented(async move {
                network_reconciler::watch_traefik_events(app_state).await;
                Ok::<(), anyhow::Error>(())
            });
        }
    } else {
        tracing::info!(
            "Reconciling proxy networks on every app check only (traefik.watch_docker_events is disabled)"
        );
    }

    // Setup the scheduler to check for running apps.
    let stop_flag = app_state.clone().stop_flag.clone();
    let mut scheduler = clokwerk::AsyncScheduler::new();

    // Check for running apps every x seconds.
    {
        let app_state = app_state.clone();
        scheduler
            .every(
                app_state
                    .settings
                    .scheduler
                    .running_app_check
                    .clone()
                    .into(),
            )
            .run(move || {
                let app_state = app_state.clone();
                async move {
                    schedule_app_check(app_state).await;
                }
            });
    }
    {
        // Check ttl of all apps every x seconds.
        let app_state = app_state.clone();
        scheduler
            .every(app_state.settings.scheduler.ttl_check.clone().into())
            .run(move || {
                let app_state = app_state.clone();
                async move {
                    schedule_ttl_check(app_state).await;
                }
            });
    }
    {
        let app_state = app_state.clone();
        scheduler
            .every(app_state.settings.scheduler.task_cleanup.clone().into())
            .run(move || {
                let app_state = app_state.clone();
                async move {
                    app_state
                        .task_manager
                        .run_cleanup_task(app_state.settings.scheduler.task_cleanup.clone())
                        .await;
                    app_state
                        .messenger
                        .broadcast_to_all(
                            scotty_core::websocket::message::WebSocketMessage::TaskListUpdated,
                        )
                        .await;
                }
            });
    }
    {
        // Sample memory metrics every 10 seconds
        scheduler
            .every(clokwerk::Interval::Seconds(10))
            .run(move || async move {
                crate::metrics::sample_memory_metrics().await;
            });
    }
    {
        // Sample Tokio task metrics every 10 seconds
        scheduler
            .every(clokwerk::Interval::Seconds(10))
            .run(move || async move {
                crate::metrics::sample_tokio_metrics().await;
            });
    }
    {
        // Sample AppList metrics every 30 seconds
        let app_state = app_state.clone();
        scheduler
            .every(clokwerk::Interval::Seconds(30))
            .run(move || {
                let app_state = app_state.clone();
                async move {
                    crate::metrics::sample_app_list_metrics(app_state).await;
                }
            });
    }
    {
        // Clean up expired OAuth sessions every 5 minutes
        let app_state = app_state.clone();
        scheduler
            .every(clokwerk::Interval::Minutes(5))
            .run(move || {
                let app_state = app_state.clone();
                async move {
                    if let Some(oauth_state) = &app_state.oauth_state {
                        crate::oauth::cleanup::cleanup_device_flow_sessions(
                            oauth_state.device_flow_store.clone(),
                        );
                        crate::oauth::cleanup::cleanup_web_flow_sessions(
                            oauth_state.web_flow_store.clone(),
                        );
                        crate::oauth::cleanup::cleanup_oauth_sessions(
                            oauth_state.session_store.clone(),
                        );
                    }
                }
            });
    }
    {
        // Sample OAuth session counts every 30 seconds
        let app_state = app_state.clone();
        scheduler
            .every(clokwerk::Interval::Seconds(30))
            .run(move || {
                let app_state = app_state.clone();
                async move {
                    if let Some(oauth_state) = &app_state.oauth_state {
                        crate::oauth::cleanup::sample_oauth_session_metrics(
                            oauth_state.device_flow_store.clone(),
                            oauth_state.web_flow_store.clone(),
                            oauth_state.session_store.clone(),
                        );
                    }
                }
            });
    }
    // Handle the scheduler in a separate task.
    let handle = crate::metrics::spawn_instrumented({
        let stop_flag = stop_flag.clone();
        async move {
            while !stop_flag.is_stopped() {
                scheduler.run_pending().await;
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }

            Ok(())
        }
    });

    Ok(handle)
}

#[instrument(skip(app_state))]
async fn schedule_app_check(app_state: SharedAppState) {
    tracing::info!("Checking running apps");
    match find_apps(&app_state).await {
        Ok(mut apps) => {
            // Reconcile before publishing, so the app list is stored with the
            // connectivity this pass observed and clients never see it flicker
            // through `Unknown`. Reconciling only on a successful discovery is
            // also what keeps pruning safe.
            if let Err(e) =
                network_reconciler::reconcile_traefik_networks(&app_state, &mut apps).await
            {
                tracing::error!("Traefik network reconciliation failed: {:?}", e);
            }
            let _ = app_state.apps.set_apps(&apps).await;
            tracing::info!("Found {} apps", app_state.apps.len().await);
            app_state
                .messenger
                .broadcast_to_all(scotty_core::websocket::message::WebSocketMessage::AppListUpdated)
                .await;
        }
        Err(e) => {
            tracing::error!("Error while checking running apps: {:?}", e);
        }
    }
}

#[instrument(skip(app_state))]
async fn schedule_ttl_check(app_state: SharedAppState) {
    tracing::info!("Checking ttl on running apps");
    let apps = app_state.apps.get_apps().await;
    for app in apps.apps.iter() {
        match check_app_ttl(app_state.clone(), app).await {
            Ok(_) => {
                tracing::debug!("TTL check passed for app: {}", app.name);
            }
            Err(e) => {
                tracing::error!("TTL check failed for app: {} - {:?}", app.name, e);
            }
        }
    }
}
