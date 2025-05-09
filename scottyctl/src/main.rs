mod api;
mod cli;
mod commands;
mod utils;

// Export progress macros for use in other modules
#[macro_export]
macro_rules! progress_println {
    ($tracker:expr, $($arg:tt)*) => {
        $tracker.println(&format!($($arg)*)).ok();
    };
}

#[macro_export]
macro_rules! progress_print {
    ($tracker:expr, $($arg:tt)*) => {
        $tracker.print(&format!($($arg)*)).ok();
    };
}

use clap::{CommandFactory, Parser};
use cli::print_completions;
use cli::{Cli, Commands};
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, filter::EnvFilter};

pub struct ServerSettings {
    pub server: String,
    pub access_token: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer())
        .init();

    let server_settings = ServerSettings {
        server: cli.server.clone(),
        access_token: cli.access_token.clone(),
    };

    info!("Running command {:?} ...", &cli.command);

    // Removed the early return and let the match statement execute
    match &cli.command {
        Commands::List => commands::apps::list_apps(&server_settings).await?,
        Commands::Rebuild(cmd) => commands::apps::rebuild_app(&server_settings, cmd).await?,
        Commands::Start(cmd) | Commands::Run(cmd) => {
            commands::apps::run_app(&server_settings, cmd).await?
        }
        Commands::Stop(cmd) => commands::apps::stop_app(&server_settings, cmd).await?,
        Commands::Destroy(cmd) => commands::apps::destroy_app(&server_settings, cmd).await?,
        Commands::Purge(cmd) => commands::apps::purge_app(&server_settings, cmd).await?,
        Commands::Adopt(cmd) => commands::apps::adopt_app(&server_settings, cmd).await?,
        Commands::Info(cmd) => commands::apps::info_app(&server_settings, cmd).await?,
        Commands::Create(cmd) => commands::apps::create_app(&server_settings, cmd).await?,
        Commands::NotifyAdd(cmd) => {
            commands::notify::add_notification(&server_settings, cmd).await?
        }
        Commands::NotifyRemove(cmd) => {
            commands::notify::remove_notification(&server_settings, cmd).await?
        }
        Commands::Completion(cmd) => {
            let mut cli_cmd = Cli::command();
            print_completions(cmd.shell, &mut cli_cmd);
        }
        Commands::BlueprintList => commands::blueprints::list_blueprints(&server_settings).await?,
    }
    Ok(())
}
