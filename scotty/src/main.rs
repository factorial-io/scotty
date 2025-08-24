mod api;
mod app_state;
mod docker;
mod http;
mod init_telemetry;
mod notification;
mod oauth;
mod onepassword;
mod services;
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
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Parser)]
enum Commands {
    /// Show current configuration and exit
    Config,
    /// Start the scotty server (default)
    Run,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command.as_ref().unwrap_or(&Commands::Run) {
        Commands::Config => {
            let app_state = app_state::AppState::new_for_config_only().await?;
            println!("{:#?}", &app_state.settings);
            return Ok(());
        }
        Commands::Run => {
            // Continue with the normal server startup
        }
    }

    let mut handles = vec![];

    let app_state = app_state::AppState::new().await?;
    init_telemetry::init_telemetry_and_tracing(&app_state.clone().settings.telemetry)?;

    // Determine if telemetry tracing is enabled
    let telemetry_enabled = app_state
        .settings
        .telemetry
        .as_ref()
        .map(|settings| settings.to_lowercase().split(',').any(|s| s == "traces"))
        .unwrap_or(false);

    // Setup http server.
    {
        let handle = setup_http_server(
            app_state.clone(),
            &app_state.clone().settings.api.bind_address,
            telemetry_enabled,
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
