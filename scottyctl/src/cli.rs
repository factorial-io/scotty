use crate::utils::parsers::{
    parse_app_ttl, parse_basic_auth, parse_custom_domain_mapping, parse_env_vars,
    parse_folder_containing_docker_compose, parse_service_ids, parse_service_ports,
};
use clap::{Parser, Subcommand};
use clap_complete::Shell;
use scotty_core::{
    apps::app_data::{AppTtl, ServicePortMapping},
    apps::create_app_request::CustomDomainMapping,
    notification_types::NotificationReceiver,
};

#[derive(Parser)]
#[command(name = "scottyctl")]
#[command(about = "Yet another micro platform as a service controlling tool")]
#[command(version)]
pub struct Cli {
    #[arg(long, env = "SCOTTY_SERVER", default_value = "http://localhost:21342")]
    pub server: String,

    #[arg(long, env = "SCOTTY_ACCESS_TOKEN")]
    pub access_token: Option<String>,

    #[arg(long, default_value = "false")]
    pub debug: bool,

    #[arg(long, default_value = "false", help = "Bypass version compatibility check (not recommended)")]
    pub bypass_version_check: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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
    /// Run a custom action on an app
    #[command(name = "app:action")]
    Action(ActionCommand),

    /// setup notificattions to other services
    #[command(name = "notify:add")]
    NotifyAdd(NotifyAddCommand),

    /// remove notificattions to other services
    #[command(name = "notify:remove")]
    NotifyRemove(NotifyRemoveCommand),

    /// List all available blueprints
    #[command(name = "blueprint:list")]
    BlueprintList,

    /// Show detailed information about a blueprint
    #[command(name = "blueprint:info")]
    BlueprintInfo(BlueprintInfoCommand),

    /// Show shell completion script.
    #[command(name = "completion")]
    Completion(CompletionCommand),

    /// Authenticate with the Scotty server
    #[command(name = "auth:login")]
    AuthLogin(AuthLoginCommand),

    /// Logout and clear stored authentication
    #[command(name = "auth:logout")]
    AuthLogout,

    /// Show authentication status
    #[command(name = "auth:status")]
    AuthStatus,

    /// Refresh authentication token
    #[command(name = "auth:refresh")]
    AuthRefresh,

    #[command(name = "test")]
    Test,
}

#[derive(Debug, Parser)]
pub struct CompletionCommand {
    #[arg(value_enum)]
    pub shell: Shell,
}

#[derive(Debug, Parser)]
pub struct BlueprintListCommand {}

#[derive(Debug, Parser)]
pub struct BlueprintInfoCommand {
    /// Name of the blueprint
    pub blueprint_name: String,
}

#[derive(Debug, Parser)]
pub struct RunCommand {
    /// Name of the app
    pub app_name: String,
}

pub type StopCommand = RunCommand;
pub type PurgeCommand = RunCommand;
pub type AdoptCommand = RunCommand;
pub type InfoCommand = RunCommand;
pub type RebuildCommand = RunCommand;
pub type DestroyCommand = RunCommand;

#[derive(Debug, Parser)]
pub struct NotifyAddCommand {
    /// Name of the app
    pub app_name: String,

    /// List of service-ids to subscribe to.
    /// Some service-ids support additional parameters e.g.
    /// the mattermost-channel or
    /// the gitlab project-id and mergerequest-id.
    #[arg(long,value_parser=parse_service_ids, value_name="SERVICE_TYPE://SERVICE_ID/(CHANNEL|PROJECT_ID/MR_ID)")]
    pub service_id: Vec<NotificationReceiver>,
}

pub type NotifyRemoveCommand = NotifyAddCommand;

#[derive(Debug, Parser)]
pub struct ActionCommand {
    /// Name of the app
    pub app_name: String,

    /// Name of the custom action to run
    pub action_name: String,
}

#[derive(Debug, Parser)]
pub struct CreateCommand {
    /// Name of the app
    pub app_name: String,

    /// Path to the folder containing a docker-compose file and other needed files
    #[arg(name="folder", long, value_parser=parse_folder_containing_docker_compose)]
    pub docker_compose_path: String,

    /// Public service ports to expose, can be specified multiple times (e.g. web:80, api:8080)
    #[arg(long, value_parser=parse_service_ports, value_name="SERVICE:PORT", required_unless_present="app_blueprint")]
    pub service: Vec<ServicePortMapping>,

    /// Custom domain(s) to use for the app (e.g. example.com:my-service), add an option for every domain or service
    #[arg(long, value_name="DOMAIN:SERVICE", value_parser=parse_custom_domain_mapping)]
    pub custom_domain: Vec<CustomDomainMapping>,

    /// Basic auth credentials for the app (user:password)
    #[arg(long, value_parser=parse_basic_auth, value_name="USER:PASSWORD")]
    pub basic_auth: Option<(String, String)>,

    /// Path to a file containing environment variables (one KEY=VALUE per line)
    #[arg(long, value_name = "PATH")]
    pub env_file: Option<String>,

    /// Pass environment variables to the app (e.g. KEY=VALUE), use multiple times for multiple variables
    #[arg(long, value_name = "KEY=VALUE", value_parser(parse_env_vars))]
    pub env: Vec<(String, String)>,

    /// Name of private docker registry to use (Needs to be configured on server-side)
    #[arg(long)]
    pub registry: Option<String>,

    /// Name of the app blueprint to use
    #[arg(long, required_unless_present = "service")]
    pub app_blueprint: Option<String>,

    /// Time to live (ttl) for the app, can be in days, hours or forever
    #[arg(long, value_parser=parse_app_ttl, default_value="7d", value_name="<DAYS>d|<HOURS>h|FOREVER")]
    pub ttl: AppTtl,

    /// Destroy the app after TTL is reached
    #[arg(
        long,
        default_value = "false",
        help = "Destroy the app after TTL is reached"
    )]
    pub destroy_on_ttl: bool,

    /// Allow search engines to index the app
    #[arg(
        long,
        default_value = "false",
        help = "Allow search engines to index the app"
    )]
    pub allow_robots: bool,

    /// Custom Traefik middlewares to apply to the app, can be specified multiple times
    #[arg(long, value_name = "MIDDLEWARE")]
    pub middleware: Vec<String>,
}

#[derive(Debug, Parser)]
pub struct AuthLoginCommand {
    /// Use a specific OAuth provider URL
    #[arg(long)]
    pub provider_url: Option<String>,

    /// Skip browser opening (just show URL)
    #[arg(long, default_value = "false")]
    pub no_browser: bool,

    /// Timeout in seconds for device flow
    #[arg(long, default_value = "300")]
    pub timeout: u64,
}

pub fn print_completions<G: clap_complete::Generator>(gen: G, cmd: &mut clap::Command) {
    clap_complete::generate(gen, cmd, cmd.get_name().to_string(), &mut std::io::stdout());
}
