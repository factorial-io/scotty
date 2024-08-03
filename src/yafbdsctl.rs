mod apps;
mod init_telemetry;
mod tasks;
mod utils;

use anyhow::Context;
use apps::{
    app_data::{AppData, AppSettings, ServicePortMapping},
    create_app_request::CreateAppRequest,
    file_list::{File, FileList},
    shared_app_list::AppDataVec,
};
use base64::prelude::*;
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
use walkdir::WalkDir;

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
    /// Rebuild an app
    Rebuild(RebuildCommand),
    /// Run an installed app
    Run(RunCommand),
    /// Start an installed app, alias for run
    Start(RunCommand),
    /// Stop an installed app
    Stop(StopCommand),
    /// Remove an installed app
    Purge(PurgeCommand),
    /// Get info of an installed app
    Info(InfoCommand),
    /// Add a new app
    Create(CreateCommand),
}

#[derive(Debug, Parser)]
struct RunCommand {
    app_name: String,
}

type StopCommand = RunCommand;
type PurgeCommand = RunCommand;
type InfoCommand = RunCommand;
type RebuildCommand = RunCommand;

#[derive(Debug, Parser)]
struct CreateCommand {
    app_name: String,
    #[arg(name="folder", long, value_parser=parse_folder_containing_docker_compose)]
    docker_compose_path: String,
    #[arg(long)]
    service: Vec<String>,
}

fn parse_folder_containing_docker_compose(s: &str) -> Result<String, String> {
    let path = std::path::Path::new(s);
    if path.is_dir() && (path.join("docker-compose.yml").exists()) {
        Ok(path
            .join("docker-compose.yml")
            .to_string_lossy()
            .to_string())
    } else if path.is_dir() && (path.join("docker-compose.yaml").exists()) {
        Ok(path
            .join("docker-compose.yaml")
            .to_string_lossy()
            .to_string())
    } else {
        Err("Folder does not contain a docker-compose.yml file".to_string())
    }
}

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
        Commands::Rebuild(cmd) => {
            call_apps_api(&cli.server, "rebuild", &cmd.app_name).await?;
        }
        Commands::Start(cmd) | Commands::Run(cmd) => {
            call_apps_api(&cli.server, "run", &cmd.app_name).await?;
        }
        Commands::Stop(cmd) => {
            call_apps_api(&cli.server, "stop", &cmd.app_name).await?;
        }
        Commands::Purge(cmd) => {
            call_apps_api(&cli.server, "purge", &cmd.app_name).await?;
        }
        Commands::Info(cmd) => {
            info_app(&cli.server, &cmd.app_name).await?;
        }
        Commands::Create(cmd) => {
            create_app(&cli.server, cmd).await?;
        }
    }
    Ok(())
}

async fn get_or_post(
    server: &str,
    action: &str,
    method: &str,
    body: Option<serde_json::Value>,
) -> anyhow::Result<serde_json::Value> {
    let url = format!("{}/api/v1/{}", server, action);
    info!("Calling yafbds API at {}", &url);

    let client = reqwest::Client::new();
    let response = match method.to_lowercase().as_str() {
        "post" => {
            if let Some(body) = body {
                client.post(&url).json(&body).send().await?
            } else {
                client.post(&url).send().await?
            }
        }
        _ => client.get(&url).send().await?,
    };

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

async fn get(server: &str, method: &str) -> anyhow::Result<serde_json::Value> {
    get_or_post(server, method, "GET", None).await
}

async fn call_apps_api(server: &str, verb: &str, app_name: &str) -> anyhow::Result<()> {
    let result = get(server, &format!("apps/{}/{}", verb, app_name)).await?;
    let context: RunningAppContext =
        serde_json::from_value(result).context("Failed to parse context from API")?;
    let app_data = wait_for_task(server, &context).await?;
    print_app_info(&app_data)?;
    Ok(())
}

fn format_since(duration: &Option<TimeDelta>) -> String {
    match duration {
        Some(d) => format_chrono_duration(d),
        None => "N/A".to_string(),
    }
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
            eprint!("{}", partial_output.blue());
        }
        // Handle stdout
        {
            let partial_output = task.stdout[last_position..].to_string();
            last_position = task.stdout.len();
            print!("{}", partial_output.blue());
        }

        // Check if task is done
        done = task.state != State::Running;
        if !done {
            // Sleep for half a second
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
    let app_data = get(server, &format!("apps/info/{}", &context.app_data.name)).await?;
    let app_data: AppData = serde_json::from_value(app_data).context("Failed to parse app data")?;

    Ok(app_data)
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

fn collect_files(docker_compose_path: &str) -> anyhow::Result<FileList> {
    let folder = std::path::Path::new(docker_compose_path)
        .parent()
        .unwrap()
        .to_str()
        .unwrap();
    let mut files = vec![];
    for entry in WalkDir::new(folder) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.path().to_str().unwrap().to_string();
            let content = std::fs::read_to_string(&path)?;
            let relative_path = path.replace(folder, ".");
            files.push(File {
                name: relative_path,
                content: content.as_bytes().to_vec(),
            });
        }
    }
    Ok(FileList { files })
}
async fn create_app(server: &str, cmd: &CreateCommand) -> anyhow::Result<()> {
    let file_list = collect_files(&cmd.docker_compose_path)?;
    // Encode content base64
    let file_list = FileList {
        files: file_list
            .files
            .iter()
            .map(|f| File {
                name: f.name.clone(),
                content: BASE64_STANDARD.encode(&f.content).into(),
            })
            .collect(),
    };
    let services = cmd
        .service
        .iter()
        .map(|s| {
            let splits: Vec<&str> = s.split("=").collect();
            ServicePortMapping {
                service: splits[0].to_string(),
                port: splits[1].parse().unwrap(),
            }
        })
        .collect::<Vec<ServicePortMapping>>();
    let payload = CreateAppRequest {
        app_name: cmd.app_name.clone(),
        settings: AppSettings {
            needs_setup: true,
            public_services: services,
            ..Default::default()
        },
        files: file_list,
    };
    println!("Payload: {:?}", payload);
    let result = get_or_post(
        server,
        "apps/create",
        "POST",
        Some(serde_json::to_value(payload).unwrap()),
    )
    .await?;

    let context: RunningAppContext =
        serde_json::from_value(result).context("Failed to parse context from API")?;
    println!("RunninAppContext: {:?}", context);

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
