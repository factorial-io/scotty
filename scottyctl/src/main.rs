use anyhow::Context;
use apps::{
    app_data::{AppData, AppSettings, AppStatus, AppTtl, ServicePortMapping},
    create_app_request::{CreateAppRequest, CustomDomainMapping},
    file_list::{File, FileList},
    shared_app_list::AppDataVec,
};
use base64::prelude::*;
use chrono::TimeDelta;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use notification_types::{
    GitlabContext, MattermostContext, NotificationReceiver, RemoveNotificationRequest,
    WebhookContext,
};
use owo_colors::OwoColorize;
use scotty_core::{apps, notification_types, settings, tasks, utils};
use settings::app_blueprint::AppBlueprintList;
use tabled::{
    builder::Builder,
    settings::{object::Columns, Style, Width},
};
use tasks::{
    running_app_context::RunningAppContext,
    task_details::{State, TaskDetails},
};
use tracing::info;
use utils::format::format_bytes;
use utils::format::format_chrono_duration;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "scottyctl")]
#[command(about = "Yet another micro platform as a service controlling tool")]
#[command(version)]
struct Cli {
    #[arg(long, env = "SCOTTY_SERVER", default_value = "http://localhost:21342")]
    server: String,

    #[arg(long, env = "SCOTTY_ACCESS_TOKEN")]
    access_token: Option<String>,

    #[arg(long, default_value = "false")]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all installed apps
    #[command(name = "app:list")]
    List,
    /// Rebuild an app
    #[command(name = "app:rebuild")]
    Rebuild(RebuildCommand),
    /// Run an installed app
    #[command(name = "app:run")]
    Run(RunCommand),
    /// Start an installed app, alias for run
    #[command(name = "app:start")]
    Start(RunCommand),
    /// Stop an installed app
    #[command(name = "app:stop")]
    Stop(StopCommand),
    /// Purge an installed app
    #[command(name = "app:purge")]
    Purge(PurgeCommand),
    /// Adopt a docker-compose based app to be controlled by scotty
    #[command(name = "app:adopt")]
    Adopt(AdoptCommand),
    /// Get info of an installed app
    #[command(name = "app:info")]
    Info(InfoCommand),
    /// Add a new app
    #[command(name = "app:create")]
    Create(Box<CreateCommand>),
    /// Destroy an app
    #[command(name = "app:destroy")]
    Destroy(DestroyCommand),

    /// setup notificattions to other services
    #[command(name = "notify:add")]
    NotifyAdd(NotifyAddCommand),

    /// remove notificattions to other services
    #[command(name = "notify:remove")]
    NotifyRemove(NotifyRemoveCommand),

    /// List all available blueprints
    #[command(name = "blueprint:list")]
    BlueprintList,

    /// Show shell completion script.
    #[command(name = "completion")]
    Completion(CompletionCommand),
}

#[derive(Debug, Parser)]
struct CompletionCommand {
    #[arg(value_enum)]
    shell: Shell,
}
#[derive(Debug, Parser)]
struct BlueprintListCommand {}

impl clap_complete::Generator for &CompletionCommand {
    fn file_name(&self, name: &str) -> String {
        self.shell.file_name(name)
    }

    fn generate(&self, cmd: &clap::Command, buf: &mut dyn std::io::Write) {
        self.shell.generate(cmd, buf)
    }
}

#[derive(Debug, Parser)]
struct RunCommand {
    /// Name of the app
    app_name: String,
}

type StopCommand = RunCommand;
type PurgeCommand = RunCommand;
type AdoptCommand = RunCommand;
type InfoCommand = RunCommand;
type RebuildCommand = RunCommand;
type DestroyCommand = RunCommand;

#[derive(Debug, Parser)]
struct NotifyAddCommand {
    /// Name of the app
    app_name: String,

    /// List of service-ids to subscribe to.
    /// Some service-ids support additional parameters e.g.
    /// the mattermost-channel or
    /// the gitlab project-id and mergerequest-id.
    #[arg(long,value_parser=parse_service_ids, value_name="SERVICE_TYPE://SERVICE_ID/(CHANNEL|PROJECT_ID/MR_ID)")]
    service_id: Vec<NotificationReceiver>,
}

type NotifyRemoveCommand = NotifyAddCommand;

#[derive(Debug, Parser)]
struct CreateCommand {
    /// Name of the app
    app_name: String,

    /// Path to the folder containing a docker-compose file and other needed files
    #[arg(name="folder", long, value_parser=parse_folder_containing_docker_compose)]
    docker_compose_path: String,

    /// Public service ports to expose, can be specified multiple times (e.g. web:80, api:8080)
    #[arg(long, value_parser=parse_service_ports, value_name="SERVICE:PORT", required_unless_present="app_blueprint")]
    service: Vec<ServicePortMapping>,

    /// Custom domain(s) to use for the app (e.g. example.com:my-service), add an option for every domain or service
    #[arg(long, value_name="DOMAIN:SERVICE", value_parser=parse_custom_domain_mapping)]
    custom_domain: Vec<CustomDomainMapping>,

    /// Basic auth credentials for the app (user:password)
    #[arg(long, value_parser=parse_basic_auth, value_name="USER:PASSWORD")]
    basic_auth: Option<(String, String)>,

    /// Pass environment variables to the app (e.g. KEY=VALUE), use multiple times for multiple variables
    #[arg(long, value_name = "KEY=VALUE", value_parser(parse_env_vars))]
    env: Vec<(String, String)>,

    /// Name of private docker registry to use (Needs to be configured on server-side)
    #[arg(long)]
    registry: Option<String>,

    /// Name of the app blueprint to use
    #[arg(long, required_unless_present = "service")]
    app_blueprint: Option<String>,

    /// Time to live (ttl) for the app, can be in days, hours or forever
    #[arg(long, value_parser=parse_app_ttl, default_value="7d", value_name="<DAYS>d|<HOURS>h|FOREVER")]
    ttl: AppTtl,

    #[arg(
        long,
        default_value = "false",
        help = "Allow search engines to index the app"
    )]
    allow_robots: bool,
}

struct ServerSettings {
    server: String,
    access_token: Option<String>,
}

fn print_completions<G: Generator>(gen: G, cmd: &mut clap::Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut std::io::stdout());
}

fn parse_service_ids(s: &str) -> Result<NotificationReceiver, String> {
    let parts: Vec<&str> = s.split("://").collect();

    if parts.len() < 2 {
        return Err("Invalid service ID format".to_string());
    }
    let service_type = parts[0];

    let parts = parts[1].split("/").collect::<Vec<&str>>();
    if parts.is_empty() {
        return Err("Invalid service ID format".to_string());
    }
    let service_id = parts[0];

    match service_type {
        "log" => Ok(NotificationReceiver::Log),
        "webhook" => {
            if parts.len() != 1 {
                return Err("Invalid service ID format for webhook".to_string());
            }
            Ok(NotificationReceiver::Webhook(WebhookContext {
                service_id: service_id.to_string(),
            }))
        }
        "mattermost" => {
            if parts.len() != 2 {
                return Err("Invalid service ID format for mattermost".to_string());
            }
            let channel = parts[1];
            Ok(NotificationReceiver::Mattermost(MattermostContext {
                service_id: service_id.to_string(),
                channel: channel.to_string(),
            }))
        }
        "gitlab" => {
            if parts.len() < 3 {
                return Err("Invalid service ID format for gitlab".to_string());
            }
            let project_id = parts[1..parts.len() - 1].join("/").to_string();
            let mr_id = parts.last().unwrap().parse::<u64>().unwrap();
            Ok(NotificationReceiver::Gitlab(GitlabContext {
                service_id: service_id.to_string(),
                project_id,
                mr_id,
            }))
        }
        _ => Err(format!(
            "Unknown service type {}, allowed values are log, mattermost, webhook and gitlab",
            service_type
        )),
    }
}

fn parse_app_ttl(s: &str) -> Result<AppTtl, String> {
    if s.eq_ignore_ascii_case("forever") {
        return Ok(AppTtl::Forever);
    }
    if let Some(days) = s.strip_suffix("d") {
        if let Ok(num_days) = days.parse::<u32>() {
            return Ok(AppTtl::Days(num_days));
        }
    }
    if let Some(hours) = s.strip_suffix("h") {
        if let Ok(num_hours) = hours.parse::<u32>() {
            return Ok(AppTtl::Hours(num_hours)); // Assuming AppTtl has a variant called `Hours`
        }
    }
    Err(format!("Invalid TTL format: {}", s))
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

fn parse_basic_auth(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid basic auth format, should be user:password".to_string());
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn parse_custom_domain_mapping(s: &str) -> Result<CustomDomainMapping, String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid custom domain format, should be domain:service".to_string());
    }
    Ok(CustomDomainMapping {
        domain: parts[0].to_string(),
        service: parts[1].to_string(),
    })
}

fn parse_service_ports(s: &str) -> Result<ServicePortMapping, String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid service port format, should be service:port".to_string());
    }
    let port = parts[1]
        .parse::<u32>()
        .map_err(|_| "Invalid port number".to_string())?;
    Ok(ServicePortMapping {
        service: parts[0].to_string(),
        port,
        domains: vec![],
    })
}

fn parse_env_vars(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.split('=').collect();
    if parts.len() != 2 {
        return Err("Invalid env var format, should be key=value".to_string());
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let server_settings = ServerSettings {
        server: cli.server.clone(),
        access_token: cli.access_token.clone(),
    };

    match &cli.command {
        Commands::List => {
            list_apps(&server_settings).await?;
        }
        Commands::Rebuild(cmd) => {
            call_apps_api(&server_settings, "rebuild", &cmd.app_name).await?;
        }
        Commands::Start(cmd) | Commands::Run(cmd) => {
            call_apps_api(&server_settings, "run", &cmd.app_name).await?;
        }
        Commands::Stop(cmd) => {
            call_apps_api(&server_settings, "stop", &cmd.app_name).await?;
        }
        Commands::Destroy(cmd) => {
            let result = get(&server_settings, &format!("apps/destroy/{}", &cmd.app_name)).await?;
            let context: RunningAppContext =
                serde_json::from_value(result).context("Failed to parse context from API")?;
            wait_for_task(&server_settings, &context).await?;

            println!("App {} destroyed", &cmd.app_name);
        }
        Commands::Purge(cmd) => {
            call_apps_api(&server_settings, "purge", &cmd.app_name).await?;
        }
        Commands::Adopt(cmd) => {
            adopt_app(&server_settings, &cmd.app_name).await?;
        }
        Commands::Info(cmd) => {
            info_app(&server_settings, &cmd.app_name).await?;
        }
        Commands::Create(cmd) => {
            create_app(&server_settings, cmd).await?;
        }
        Commands::NotifyAdd(cmd) => {
            add_notification(&server_settings, cmd).await?;
        }
        Commands::NotifyRemove(cmd) => {
            remove_notification(&server_settings, cmd).await?;
        }
        Commands::Completion(shell) => {
            let mut cmd = Cli::command();
            print_completions(shell, &mut cmd);
        }
        Commands::BlueprintList => {
            list_blueprints(&server_settings).await?;
        }
    }
    Ok(())
}

async fn get_or_post(
    server: &ServerSettings,
    action: &str,
    method: &str,
    body: Option<serde_json::Value>,
) -> anyhow::Result<serde_json::Value> {
    let url = format!("{}/api/v1/{}", server.server, action);
    info!("Calling scotty API at {}", &url);

    let client = reqwest::Client::new();
    let response = match method.to_lowercase().as_str() {
        "post" => {
            if let Some(body) = body {
                client.post(&url).json(&body)
            } else {
                client.post(&url)
            }
        }
        _ => client.get(&url),
    };

    let response = response
        .bearer_auth(server.access_token.as_deref().unwrap_or_default())
        .send()
        .await
        .context(format!("Failed to call scotty API at {}", &url))?;

    if response.status().is_success() {
        let json = response.json::<serde_json::Value>().await.context(format!(
            "Failed to parse response from scotty API at {}",
            &url
        ))?;
        Ok(json)
    } else {
        let status = &response.status();
        let content = response.json::<serde_json::Value>().await.ok();
        let error_message = if let Some(content) = content {
            if let Some(message) = content.get("message") {
                format!(": {}", message.as_str().unwrap_or("Unknown error"))
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        Err(anyhow::anyhow!(
            "Failed to call scotty API at {} : {}{}",
            &url,
            &status,
            error_message
        ))
    }
}

async fn get(server: &ServerSettings, method: &str) -> anyhow::Result<serde_json::Value> {
    get_or_post(server, method, "GET", None).await
}

async fn call_apps_api(server: &ServerSettings, verb: &str, app_name: &str) -> anyhow::Result<()> {
    let result = get(server, &format!("apps/{}/{}", verb, app_name)).await?;
    let context: RunningAppContext =
        serde_json::from_value(result).context("Failed to parse context from API")?;
    wait_for_task(server, &context).await?;
    let app_data = get_app_info(server, &context.app_data.name).await?;
    print_app_info(&app_data)?;
    Ok(())
}

fn format_since(duration: &Option<TimeDelta>) -> String {
    match duration {
        Some(d) => format_chrono_duration(d),
        None => "N/A".to_string(),
    }
}

async fn wait_for_task(server: &ServerSettings, context: &RunningAppContext) -> anyhow::Result<()> {
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

        if let Some(exit_code) = task.last_exit_code {
            if done && exit_code != 0 {
                return Err(anyhow::anyhow!("Task failed with exit code {}", exit_code));
            }
        }
    }

    Ok(())
}

async fn get_app_info(server: &ServerSettings, app_name: &str) -> anyhow::Result<AppData> {
    let app_data = get(server, &format!("apps/info/{}", app_name)).await?;
    let app_data: AppData = serde_json::from_value(app_data).context("Failed to parse app data")?;

    Ok(app_data)
}

fn colored_by_status(name: &str, status: &AppStatus) -> String {
    match status {
        AppStatus::Starting | AppStatus::Running => name.green().to_string(),
        AppStatus::Stopped => name.blue().to_string(),
        AppStatus::Creating => name.bright_green().to_string(),
        AppStatus::Destroying => name.bright_red().to_string(),
        AppStatus::Unsupported => name.white().to_string(),
    }
}

async fn list_apps(server: &ServerSettings) -> anyhow::Result<()> {
    let result = get(server, "apps/list").await?;

    let apps: AppDataVec = serde_json::from_value(result).context("Failed to parse apps list")?;

    let mut builder = Builder::default();
    builder.push_record(vec!["Name", "Status", "Since", "URLs"]);
    for app in apps.apps {
        let urls = app.urls();
        builder.push_record(vec![
            &colored_by_status(&app.name, &app.status),
            &app.status.to_string(),
            &format_since(&app.running_since()),
            &urls.join("\n"),
        ]);
    }

    let mut table = builder.build();

    table.with(Style::rounded());

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
            let file_name = entry.file_name().to_str().unwrap();
            if file_name == ".DS_Store" || entry.path().to_str().unwrap().contains("/.git/") {
                info!("Ignoring file {:?}", entry);
                continue;
            }
            info!("Reading file {:?}", entry);
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
async fn create_app(server: &ServerSettings, cmd: &CreateCommand) -> anyhow::Result<()> {
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

    let payload = CreateAppRequest {
        app_name: cmd.app_name.clone(),
        custom_domains: cmd.custom_domain.clone(),
        settings: AppSettings {
            public_services: cmd.service.clone(),
            basic_auth: cmd.basic_auth.clone(),
            environment: cmd.env.iter().cloned().collect(),
            registry: cmd.registry.clone(),
            app_blueprint: cmd.app_blueprint.clone(),
            time_to_live: cmd.ttl.clone(),
            disallow_robots: !cmd.allow_robots,
            ..Default::default()
        },
        files: file_list,
    };

    let payload = serde_json::to_value(&payload).context("Failed to serialize payload")?;
    let size = format_bytes(payload.to_string().len());
    println!(
        "🚀 Beaming your app {} up to {} ({})... \n",
        &cmd.app_name.yellow(),
        &server.server.yellow(),
        size.blue()
    );
    let result = get_or_post(server, "apps/create", "POST", Some(payload)).await?;

    let context: RunningAppContext =
        serde_json::from_value(result).context("Failed to parse context from API")?;

    wait_for_task(server, &context).await?;
    let app_data = get_app_info(server, &context.app_data.name).await?;

    print_app_info(&app_data)?;
    Ok(())
}

async fn info_app(server: &ServerSettings, app_name: &str) -> anyhow::Result<()> {
    let result = get(server, &format!("apps/info/{}", app_name)).await?;
    let app_data: AppData = serde_json::from_value(result)?;
    print_app_info(&app_data)?;
    Ok(())
}
async fn adopt_app(server: &ServerSettings, app_name: &str) -> anyhow::Result<()> {
    let result = get(server, &format!("apps/adopt/{}", app_name)).await?;
    let app_data: AppData = serde_json::from_value(result)?;
    print_app_info(&app_data)?;
    Ok(())
}

fn print_app_info(app_data: &AppData) -> anyhow::Result<()> {
    let mut builder = Builder::default();
    builder.push_record(vec!["Service", "Status", "Running since", "URL"]);
    for service in &app_data.services {
        let urls = service.get_urls();
        builder.push_record(vec![
            &service.service,
            &service.status.to_string(),
            &format_since(&service.running_since()),
            &urls.join("\n"),
        ]);
    }

    let mut table = builder.build();
    table.with(Style::rounded());

    println!("Info for {}", app_data.name);
    println!("{}", table);

    if app_data.settings.is_some() && !app_data.settings.as_ref().unwrap().notify.is_empty() {
        println!("Notification services");
        let mut builder = Builder::default();
        builder.push_record(["Type", "Service-Id", "Context"]);
        for notification in &app_data.settings.as_ref().unwrap().notify {
            #[allow(unused_assignments)]
            let mut context: String = "".into();
            builder.push_record(match notification {
                NotificationReceiver::Log => ["Log", "Log", ""],
                NotificationReceiver::Webhook(ctx) => ["Webhook", &ctx.service_id, ""],
                NotificationReceiver::Mattermost(ctx) => {
                    ["Mattermost", &ctx.service_id, &ctx.channel]
                }
                NotificationReceiver::Gitlab(ctx) => {
                    context = format!("Project-Id: {}  MR-Id: {}", ctx.project_id, ctx.mr_id);
                    ["Gitlab", &ctx.service_id, &context]
                }
            });
        }
        println!("{}", builder.build().with(Style::rounded()));
    }

    Ok(())
}

async fn add_notification(server: &ServerSettings, cmd: &NotifyAddCommand) -> anyhow::Result<()> {
    let payload = serde_json::json!({
        "app_name": cmd.app_name,
        "service_ids": cmd.service_id,
    });

    let result = get_or_post(server, "apps/notify/add", "POST", Some(payload)).await?;

    let app_data: AppData =
        serde_json::from_value(result).context("Failed to parse context from API")?;

    print_app_info(&app_data)?;
    Ok(())
}

async fn remove_notification(
    server: &ServerSettings,
    cmd: &NotifyRemoveCommand,
) -> anyhow::Result<()> {
    let payload = RemoveNotificationRequest {
        app_name: cmd.app_name.clone(),
        service_ids: cmd.service_id.clone(),
    };

    let payload = serde_json::to_value(&payload).context("Failed to serialize payload")?;
    let result = get_or_post(server, "apps/notify/remove", "POST", Some(payload)).await?;

    let app_data: AppData =
        serde_json::from_value(result).context("Failed to parse context from API")?;

    print_app_info(&app_data)?;
    Ok(())
}

async fn list_blueprints(server: &ServerSettings) -> anyhow::Result<()> {
    let result = get(server, "blueprints").await?;
    let blueprints: AppBlueprintList = serde_json::from_value(result)?;

    let mut builder = Builder::default();
    builder.push_record(vec!["Id", "Name", "Description", "Required Services"]);
    for blueprint in blueprints.blueprints {
        let id = blueprint.0;
        let blueprint = blueprint.1;
        builder.push_record(vec![
            &id,
            &blueprint.name,
            &blueprint.description,
            &blueprint.required_services.join(", "),
        ]);
    }

    let mut table = builder.build();
    table.with(Style::modern_rounded());
    table.modify(Columns::single(0), Width::wrap(15).keep_words(true));
    table.modify(Columns::single(1), Width::wrap(15).keep_words(true));
    table.modify(Columns::single(3), Width::wrap(10).keep_words(true));

    println!("{}", table);
    Ok(())
}
