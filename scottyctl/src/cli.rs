use crate::utils::parsers::{
    parse_app_ttl, parse_basic_auth, parse_custom_domain_mapping, parse_env_vars,
    parse_folder_containing_docker_compose, parse_service_ids, parse_service_ports,
};
use clap::{Parser, Subcommand};
use clap_complete::Shell;
use scotty_core::{
    admin::{
        CreateAssignmentRequest, CreateRoleRequest, CreateScopeRequest, GetUserPermissionsRequest,
        RemoveAssignmentRequest, TestPermissionRequest,
    },
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

    #[arg(
        long,
        default_value = "false",
        help = "Bypass version compatibility check (not recommended)"
    )]
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
    /// View logs for an app service
    #[command(name = "app:logs")]
    Logs(LogsCommand),
    /// Open interactive shell for an app service
    #[command(name = "app:shell")]
    Shell(ShellCommand),

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

    /// List all authorization scopes
    #[command(name = "admin:scopes:list")]
    AdminScopesList,

    /// Create a new authorization scope
    #[command(name = "admin:scopes:create")]
    AdminScopesCreate(CreateScopeRequest),

    /// List all authorization roles
    #[command(name = "admin:roles:list")]
    AdminRolesList,

    /// Create a new authorization role
    #[command(name = "admin:roles:create")]
    AdminRolesCreate(CreateRoleRequest),

    /// List all user assignments
    #[command(name = "admin:assignments:list")]
    AdminAssignmentsList,

    /// Create a new user assignment
    #[command(name = "admin:assignments:create")]
    AdminAssignmentsCreate(CreateAssignmentRequest),

    /// Remove a user assignment
    #[command(name = "admin:assignments:remove")]
    AdminAssignmentsRemove(RemoveAssignmentRequest),

    /// List all available permissions
    #[command(name = "admin:permissions:list")]
    AdminPermissionsList,

    /// Test permission for a user on an app
    #[command(name = "admin:permissions:test")]
    AdminPermissionsTest(TestPermissionRequest),

    /// Get permissions for a specific user
    #[command(name = "admin:permissions:user")]
    AdminPermissionsUser(GetUserPermissionsRequest),

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

    /// Scope(s) to create the app in, can be specified multiple times (defaults to 'default')
    #[arg(long, value_name = "SCOPE")]
    pub scope: Vec<String>,
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

#[derive(Debug, Parser)]
pub struct LogsCommand {
    /// Name of the app
    pub app_name: String,

    /// Name of the service
    pub service_name: String,

    /// Follow log output (stream in real-time)
    #[arg(short = 'f', long = "follow", default_value = "false")]
    pub follow: bool,

    /// Number of lines to show (if not specified, show all available logs)
    #[arg(short = 'n', long = "lines")]
    pub lines: Option<usize>,

    /// Show logs since timestamp (e.g., "2h", "30m", "2023-01-01T10:00:00Z")
    #[arg(long = "since")]
    pub since: Option<String>,

    /// Show logs until timestamp (e.g., "1h", "2023-01-01T11:00:00Z")
    #[arg(long = "until")]
    pub until: Option<String>,

    /// Show timestamps in log output
    #[arg(short = 't', long = "timestamps")]
    pub timestamps: bool,
}

#[derive(Debug, Parser)]
pub struct ShellCommand {
    /// Name of the app
    pub app_name: String,

    /// Name of the service
    pub service_name: String,

    /// Command to execute instead of interactive shell
    #[arg(short = 'c', long = "command")]
    pub command: Option<String>,

    /// Shell to use (default: /bin/bash)
    #[arg(long = "shell")]
    pub shell: Option<String>,

    // /// User to run shell as (default: container default)
    // #[arg(short = 'u', long = "user")]
    // pub user: Option<String>,

    /// Working directory (default: container default)
    #[arg(short = 'w', long = "workdir")]
    pub workdir: Option<String>,
}

pub fn print_completions<G: clap_complete::Generator>(gen: G, cmd: &mut clap::Command) {
    clap_complete::generate(gen, cmd, cmd.get_name().to_string(), &mut std::io::stdout());
}
