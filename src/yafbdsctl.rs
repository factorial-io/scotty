mod apps;

use apps::{app_data::AppData, shared_app_list::AppDataVec};
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use tabled::{
    builder::Builder,
    settings::{object::Columns, Style},
};

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
    Run(RunCommand),
    /// Stop an installed app
    Stop(StopCommand),
    /// Remove an installed app
    Rm(RmCommand),
    /// Add a new app
    Add,
}

#[derive(Debug, Parser)]
struct RunCommand {
    app_name: String,
}

type StopCommand = RunCommand;
type RmCommand = RunCommand;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::List => {
            list_apps(&cli.server).await?;
        }
        Commands::Run(cmd) => {
            run_app(&cli.server, &cmd.app_name).await?;
        }
        Commands::Stop(cmd) => {
            stop_app(&cli.server, &cmd.app_name).await?;
        }
        Commands::Rm(cmd) => {
            rm_app(&cli.server, &cmd.app_name).await?;
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

    let mut builder = Builder::default();
    builder.push_record(vec!["Name", "Status", "URLs"]);
    for app in apps.apps {
        let urls = app.urls();
        builder.push_record(vec![app.name, app.status.to_string(), urls.join(", ")]);
    }

    let mut table = builder.build();

    table.with(Style::rounded()).modify(
        Columns::single(0),
        tabled::settings::Format::content(|s| s.blue().to_string()),
    );

    println!("{}", table);

    Ok(())
}

async fn run_app(server: &str, app_name: &str) -> anyhow::Result<()> {
    let result = get(server, &format!("apps/run/{}", app_name)).await?;
    let app_data: AppData = serde_json::from_value(result)?;
    print_app_info(&app_data)?;
    Ok(())
}

async fn stop_app(server: &str, app_name: &str) -> anyhow::Result<()> {
    let result = get(server, &format!("apps/stop/{}", app_name)).await?;
    let app_data: AppData = serde_json::from_value(result)?;
    print_app_info(&app_data)?;
    Ok(())
}

async fn rm_app(server: &str, app_name: &str) -> anyhow::Result<()> {
    let result = get(server, &format!("apps/rm/{}", app_name)).await?;
    let app_data: AppData = serde_json::from_value(result)?;
    print_app_info(&app_data)?;
    Ok(())
}

fn print_app_info(app_data: &AppData) -> anyhow::Result<()> {
    let mut builder = Builder::default();
    builder.push_record(vec!["Service", "Status", "URL"]);
    for service in &app_data.services {
        let url: String = service.get_url().unwrap_or("None".into());
        builder.push_record(vec![&service.service, &service.status.to_string(), &url]);
    }

    let mut table = builder.build();

    table.with(Style::rounded()).modify(
        Columns::single(0),
        tabled::settings::Format::content(|s| s.blue().to_string()),
    );

    println!("{}", table);

    Ok(())
}
