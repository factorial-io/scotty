mod api;
mod app_state;
mod docker;
mod http;
mod init_telemetry;
mod notification;
mod oauth;
mod onepassword;
mod settings;
mod state_machine;
mod static_files;
mod stop_flag;
mod tasks;
mod utils;

use docker::setup::setup_docker_integration;
use http::setup_http_server;
use tokio::time::sleep;
use tracing::info;

use clap::Parser;

#[derive(Parser)]
#[command(name = "scotty")]
#[command(about = "Yet another micro platform as a service")]
#[clap(version)]
struct Cli {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _cli = Cli::parse();

    let mut handles = vec![];

    let app_state = app_state::AppState::new().await?;
    init_telemetry::init_telemetry_and_tracing(&app_state.clone().settings.telemetry)?;

    // Setup http server.
    {
        let handle = setup_http_server(
            app_state.clone(),
            &app_state.clone().settings.api.bind_address,
        )
        .await?;

        handles.push(handle);
    }

    // Setup docker integration
    {
        let handle = setup_docker_integration(app_state.clone()).await?;
        handles.push(handle);
    }

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
