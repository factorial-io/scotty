mod api;
mod cli;
mod commands;
mod utils;

use clap::{CommandFactory, Parser};
use cli::print_completions;
use cli::{Cli, Commands};
use tracing::info;

pub struct ServerSettings {
    pub server: String,
    pub access_token: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
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
