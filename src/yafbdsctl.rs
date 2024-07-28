mod apps;
mod init_telemetry;
mod tasks;
mod utils;

use anyhow::Context;
use apps::{app_data::AppData, shared_app_list::AppDataVec};
use chrono::TimeDelta;
use clap::{Parser, Subcommand};
use init_telemetry::init_telemetry_and_tracing;
use owo_colors::OwoColorize;
use tabled::{
    builder::Builder,
    settings::{object::Columns, Style},
};
use tasks::{
    running_app_context::RunningAppContext,
    task_details::{State, TaskDetails},
};
use tracing::info;
use utils::format_chrono_duration;

#[derive(Parser)]
#[command(name = "yafbdsctl")]
#[command(about = "Yet another feature based deployment service control tool")]
struct Cli {
    #[arg(long, env = "YAFBDS_SERVER", default_value = "http://localhost:21342")]
    server: String,
    #[arg(long, default_value = "false")]
    debug: bool,
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
    /// Get info of an installed app
    Info(InfoCommand),
    /// Add a new app
    Add,
}

#[derive(Debug, Parser)]
struct RunCommand {
    app_name: String,
}

type StopCommand = RunCommand;
type RmCommand = RunCommand;
type InfoCommand = RunCommand;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let tracing_option = match cli.debug {
        true => Some("traces".to_string()),
        false => None,
    };
    init_telemetry_and_tracing(&tracing_option)?;

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
        Commands::Info(cmd) => {
            info_app(&cli.server, &cmd.app_name).await?;
        }
        Commands::Add => {
            unimplemented!();
        }
    }
    Ok(())
}

async fn get(server: &str, action: &str) -> anyhow::Result<serde_json::Value> {
    let url = format!("{}/api/v1/{}", server, action);
    info!("Calling yafbds API at {}", &url);
    let response = reqwest::get(&url).await?;

    if response.status().is_success() {
        let json = response.json::<serde_json::Value>().await.context(format!(
            "Failed to parse response from yafbds API at {}",
            &url
        ))?;
        Ok(json)
    } else {
        Err(anyhow::anyhow!(
            "Failed to call yafbds API at {} : {}",
            &url,
            response.status()
        ))
    }
}
fn format_since(duration: &Option<TimeDelta>) -> String {
    match duration {
        Some(d) => format_chrono_duration(d),
        None => "N/A".to_string(),
    }
}
async fn list_apps(server: &str) -> anyhow::Result<()> {
    let result = get(server, "apps/list").await?;

    let apps: AppDataVec = serde_json::from_value(result).context("Failed to parse apps list")?;

    let mut builder = Builder::default();
    builder.push_record(vec!["Name", "Status", "Since", "URLs"]);
    for app in apps.apps {
        let urls = app.urls();
        builder.push_record(vec![
            &app.name,
            &app.status.to_string(),
            &format_since(&app.running_since()),
            &urls.join("\n"),
        ]);
    }

    let mut table = builder.build();

    table.with(Style::rounded()).modify(
        Columns::single(0),
        tabled::settings::Format::content(|s| s.blue().to_string()),
    );

    println!("{}", table);

    Ok(())
}

async fn wait_for_task(server: &str, context: &RunningAppContext) -> anyhow::Result<AppData> {
    let mut done = false;
    let mut last_position = 0;
    let mut last_err_position = 0;
    while !done {
        let result = get(server, &format!("task/{}", &context.task.id)).await?;

        let task: TaskDetails = serde_json::from_value(result).context("Failed to parse task")?;

        // Handle stderr
        {
            let partial_output = task.stderr[last_err_position..].to_string();
            last_err_position = task.stderr.len();
            eprint!("{}", partial_output);
        }
        // Handle stdout
        {
            let partial_output = task.stdout[last_position..].to_string();
            last_position = task.stdout.len();
            print!("{}", partial_output);
        }

        // Check if task is done
        done = task.state != State::Running;
        if !done {
            // Sleep for half a second
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        } else {
            let app_data = get(server, &format!("apps/info/{}", &context.app_data.name)).await?;
            let app_data: AppData =
                serde_json::from_value(app_data).context("Failed to parse app data")?;
            return Ok(app_data);
        }
    }
    Err(anyhow::Error::msg("Task is still running"))
}

async fn run_app(server: &str, app_name: &str) -> anyhow::Result<()> {
    let result = get(server, &format!("apps/run/{}", app_name)).await?;
    let context: RunningAppContext =
        serde_json::from_value(result).context("Failed to parse context from API")?;
    let app_data = wait_for_task(server, &context).await?;
    print_app_info(&app_data)?;
    Ok(())
}

async fn stop_app(server: &str, app_name: &str) -> anyhow::Result<()> {
    let result = get(server, &format!("apps/stop/{}", app_name)).await?;
    let context: RunningAppContext =
        serde_json::from_value(result).context("Failed to parse context from API")?;
    let app_data = wait_for_task(server, &context).await?;
    print_app_info(&app_data)?;
    Ok(())
}

async fn rm_app(server: &str, app_name: &str) -> anyhow::Result<()> {
    let result = get(server, &format!("apps/rm/{}", app_name)).await?;
    let context: RunningAppContext =
        serde_json::from_value(result).context("Failed to parse context from API")?;
    let app_data = wait_for_task(server, &context).await?;
    print_app_info(&app_data)?;
    Ok(())
}

async fn info_app(server: &str, app_name: &str) -> anyhow::Result<()> {
    let result = get(server, &format!("apps/info/{}", app_name)).await?;
    let app_data: AppData = serde_json::from_value(result)?;
    print_app_info(&app_data)?;
    Ok(())
}

fn print_app_info(app_data: &AppData) -> anyhow::Result<()> {
    let mut builder = Builder::default();
    builder.push_record(vec!["Service", "Status", "Running since", "URL"]);
    for service in &app_data.services {
        let url: String = service.get_url().unwrap_or("None".into());
        builder.push_record(vec![
            &service.service,
            &service.status.to_string(),
            &format_since(&service.running_since()),
            &url,
        ]);
    }

    let mut table = builder.build();
    table.with(Style::rounded()).modify(
        Columns::single(0),
        tabled::settings::Format::content(|s| s.blue().to_string()),
    );

    println!("{}", table);

    Ok(())
}
