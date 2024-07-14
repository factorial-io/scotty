mod apps;

use apps::shared_app_list::AppDataVec;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "yafbdsctl")]
#[command(about = "Yet another feature based deployment service control tool")]
struct Cli {
    #[arg(long, env = "YAFBDS_SERVER", default_value = "http://localhost:21342")]
    server: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all installed apps
    List,
    /// Start an installed app
    Start,
    /// Stop an installed app
    Stop,
    /// Remove an installed app
    Rm,
    /// Add a new app
    Add,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::List => {
            list_apps(&cli.server).await?;
        }
        Commands::Start => {
            unimplemented!();
        }
        Commands::Stop => {
            unimplemented!();
        }
        Commands::Rm => {
            unimplemented!();
        }
        Commands::Add => {
            unimplemented!();
        }
    }
    Ok(())
}

async fn get(server: &str, action: &str) -> anyhow::Result<serde_json::Value> {
    let url = format!("{}/api/v1/{}", server, action);
    let response = reqwest::get(&url).await?;

    if response.status().is_success() {
        let json = response.json::<serde_json::Value>().await?;
        Ok(json)
    } else {
        Err(anyhow::anyhow!(
            "Failed to call yafbds API at {} : {}",
            &url,
            response.status()
        ))
    }
}

async fn list_apps(server: &str) -> anyhow::Result<()> {
    let result = get(server, "apps/list").await?;

    let apps: AppDataVec = serde_json::from_value(result)?;
    println!("{:?}", apps);
    Ok(())
}
