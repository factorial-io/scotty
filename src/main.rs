mod api;
mod app_state;
mod http;
mod settings;
mod stop_flag;

use http::setup_http_server;
use tokio::time::sleep;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut handles = vec![];

    let app_state = app_state::AppState::new().await?;

    let handle = setup_http_server(app_state.clone(), &app_state.settings.api.bind_address).await?;

    handles.push(handle);

    sleep(std::time::Duration::from_millis(100)).await;

    loop {
        // Remove and await completed handles
        handles.retain(|handle| !handle.is_finished());

        // Break the loop if no more handles are running
        if handles.is_empty() {
            info!("All tasks are done");
            break;
        }

        // Sleep for a short duration to avoid busy-waiting
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    Ok(())
}
