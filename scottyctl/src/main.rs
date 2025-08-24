mod api;
mod auth;
mod cli;
mod commands;
mod context;
mod preflight;
mod utils;

use clap::{CommandFactory, Parser};
use cli::print_completions;
use cli::{Cli, Commands};
use context::{AppContext, ServerSettings};
use preflight::PreflightChecker;
use tracing::info;
use tracing_subscriber::{prelude::*, EnvFilter};
use utils::tracing_layer::UiLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Create server settings from CLI parameters
    let server_settings = ServerSettings {
        server: cli.server.clone(),
        access_token: cli.access_token.clone(),
    };

    // Create the app context with UI and server settings
    let app_context = AppContext::new(server_settings);

    // Initialize tracing with our custom layer
    tracing_subscriber::registry()
        .with(UiLayer::new(app_context.ui().clone()))
        .with(EnvFilter::from_default_env())
        .init();

    info!("Running command {:?} ...", &cli.command);

    // Run preflight checks for commands that require server connection
    let needs_preflight = !matches!(
        &cli.command,
        Commands::Completion(_) | Commands::AuthLogin(_) | Commands::AuthLogout
    );

    if needs_preflight {
        let preflight =
            PreflightChecker::new(app_context.server().clone(), app_context.ui().clone());
        preflight
            .check_compatibility(cli.bypass_version_check)
            .await?;
    }

    // Execute the appropriate command with our app context
    match &cli.command {
        Commands::List => commands::apps::list_apps(&app_context).await?,
        Commands::Rebuild(cmd) => commands::apps::rebuild_app(&app_context, cmd).await?,
        Commands::Start(cmd) | Commands::Run(cmd) => {
            commands::apps::run_app(&app_context, cmd).await?
        }
        Commands::Stop(cmd) => commands::apps::stop_app(&app_context, cmd).await?,
        Commands::Destroy(cmd) => commands::apps::destroy_app(&app_context, cmd).await?,
        Commands::Purge(cmd) => commands::apps::purge_app(&app_context, cmd).await?,
        Commands::Adopt(cmd) => commands::apps::adopt_app(&app_context, cmd).await?,
        Commands::Info(cmd) => commands::apps::info_app(&app_context, cmd).await?,
        Commands::Create(cmd) => commands::apps::create_app(&app_context, cmd).await?,
        Commands::Action(cmd) => commands::apps::run_custom_action(&app_context, cmd).await?,
        Commands::NotifyAdd(cmd) => commands::notify::add_notification(&app_context, cmd).await?,
        Commands::NotifyRemove(cmd) => {
            commands::notify::remove_notification(&app_context, cmd).await?
        }
        Commands::Completion(cmd) => {
            let mut cli_cmd = Cli::command();
            print_completions(cmd.shell, &mut cli_cmd);
        }
        Commands::BlueprintList => commands::blueprints::list_blueprints(&app_context).await?,
        Commands::BlueprintInfo(cmd) => {
            commands::blueprints::blueprint_info(&app_context, cmd).await?
        }
        Commands::AuthLogin(cmd) => commands::auth::auth_login(&app_context, cmd).await?,
        Commands::AuthLogout => commands::auth::auth_logout(&app_context).await?,
        Commands::AuthStatus => commands::auth::auth_status(&app_context).await?,
        Commands::AuthRefresh => commands::auth::auth_refresh(&app_context).await?,
        Commands::Test => commands::test::run_tests(&app_context).await?,
    }
    Ok(())
}
