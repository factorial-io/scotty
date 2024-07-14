use crate::{app_state::SharedAppState, docker::find_apps::find_apps};

pub async fn setup_docker_integration(
    app_state: SharedAppState,
) -> anyhow::Result<tokio::task::JoinHandle<anyhow::Result<()>>> {
    // Find all running apps on startup.
    let _ = find_apps(&app_state).await;

    // Setup the scheduler to check for running apps.
    let stop_flag = app_state.clone().stop_flag.clone();
    let mut scheduler = clokwerk::AsyncScheduler::new();

    // Check for running apps every x seconds.
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
                tracing::info!("Checking running apps");
                match find_apps(&app_state).await {
                    Ok(apps) => {
                        let _ = app_state.apps.set_apps(&apps).await;
                        tracing::info!(
                            "Found {} apps {:?}",
                            app_state.apps.len().await,
                            &apps.apps
                        );
                    }
                    Err(e) => {
                        tracing::error!("Error while checking running apps: {:?}", e);
                    }
                }
            }
        });

    // Handle the scheduler in a separate task.
    let handle = tokio::spawn({
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
