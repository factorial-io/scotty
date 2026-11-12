//! The MCP endpoint at `/mcp`: a tools-only server over stateless
//! Streamable HTTP (rmcp). Every tool acts as the authenticated caller
//! ([`McpPrincipal`], set by [`crate::oauth::mcp::mcp_auth`]): it checks the
//! token's scope here, then calls the same `services::apps` function as the
//! REST API, which checks the caller's permissions.
//!
//! Results never carry environment values, generated parameters, basic
//! auth credentials or tokens: apps are reported as [`AppSummary`].

use std::sync::Arc;

use base64::prelude::*;
use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::{
        CacheScope, ErrorData, ListToolsResult, PaginatedRequestParams, ProtocolVersion,
        ResultType, Tool,
    },
    schemars::JsonSchema,
    service::RequestContext,
    tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        session::never::NeverSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    },
    RoleServer, ServerHandler,
};
use scotty_core::apps::{
    app_data::{AppData, AppSettings, AppTtl},
    create_app_request::CreateAppRequest,
    file_list::{File, FileList},
};
use scotty_core::output::{OutputLine, OutputStreamType};
use scotty_core::settings::loadbalancer::LoadBalancerType;
use scotty_core::tasks::task_details::State;
use scotty_core::utils::{format::format_bytes, slugify::slugify};
use serde::{Deserialize, Serialize};

use crate::app_state::SharedAppState;
use crate::oauth::mcp::{McpPrincipal, McpScope};
use crate::services::apps;
use crate::services::authorization::decision::Target;

#[derive(Clone)]
pub struct ScottyMcp {
    state: SharedAppState,
}

type ToolResult<T> = Result<Json<T>, String>;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct AppRef {
    /// The app's name, as returned by `list_apps` or `create_app`.
    pub app_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct BlueprintInfo {
    /// The blueprint id to pass to `create_app`.
    pub name: String,
    pub title: String,
    pub description: String,
    /// JSON Schema of the parameters `create_app` accepts. Properties marked
    /// `readOnly` are filled by Scotty and must not be sent.
    pub parameters: Option<serde_json::Value>,
    /// Allowed images; the first is the default.
    pub images: Vec<String>,
    /// Glob patterns for payload file paths (`*` within one directory, `**`
    /// across directories). Empty: the blueprint takes no files.
    pub payload: Vec<String>,
    /// The scopes you may create apps from this blueprint in; pass one as
    /// `scope` to `create_app`.
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct BlueprintList {
    pub blueprints: Vec<BlueprintInfo>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ServiceSummary {
    pub service: String,
    pub status: String,
    pub urls: Vec<String>,
}

/// An app without its settings' secrets: no environment, generated
/// parameters or basic auth credentials.
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct AppSummary {
    pub name: String,
    pub status: String,
    /// Public URLs of the app's services.
    pub urls: Vec<String>,
    pub services: Vec<ServiceSummary>,
    pub blueprint: Option<String>,
    pub scopes: Vec<String>,
    /// How long the app lives before Scotty stops it, e.g. `{"Days": 7}`.
    pub time_to_live: Option<serde_json::Value>,
    /// Whether the app is behind basic auth (the credentials are not shown).
    pub basic_auth: bool,
}

/// A unit enum's serde name, the same string the REST API sends, so a Rust
/// rename can't silently change the tool contract.
fn wire_name(value: &impl Serialize) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(name)) => name,
        // A variant with data: its JSON, rather than a panic in a handler.
        Ok(other) => other.to_string(),
        Err(_) => "unknown".to_string(),
    }
}

impl AppSummary {
    fn new(app: &AppData) -> Self {
        let settings = app.settings.as_ref();
        Self {
            name: app.name.clone(),
            status: app.status.to_string(),
            urls: app.urls(),
            services: app
                .services
                .iter()
                .map(|s| ServiceSummary {
                    service: s.service.clone(),
                    status: wire_name(&s.status),
                    urls: s.get_urls(),
                })
                .collect(),
            blueprint: settings.and_then(|s| s.app_blueprint.clone()),
            scopes: settings.map(|s| s.scopes.clone()).unwrap_or_default(),
            time_to_live: settings.and_then(|s| serde_json::to_value(&s.time_to_live).ok()),
            basic_auth: settings.is_some_and(|s: &AppSettings| s.basic_auth.is_some()),
        }
    }
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct AppList {
    pub apps: Vec<AppSummary>,
}

#[derive(Debug, Deserialize, JsonSchema, Default, Clone, Copy, PartialEq, Eq)]
#[schemars(crate = "rmcp::schemars")]
#[serde(rename_all = "lowercase")]
pub enum Encoding {
    /// The content is the file's text.
    #[default]
    Utf8,
    /// The content is the file's bytes, base64-encoded (for binary files).
    Base64,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct PayloadFile {
    /// Path inside the app, matching one of the blueprint's payload
    /// patterns, e.g. `html/index.html`.
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub encoding: Encoding,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct BasicAuth {
    pub user: String,
    pub password: String,
}

/// `create_app` input.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct CreateAppInput {
    /// A templated blueprint from `list_blueprints`.
    pub blueprint: String,
    /// The app name; it is slugified, the result returns the final name.
    /// Omit it to get an unguessable generated name
    /// (`<blueprint>-<10 random characters>`). An existing name is refused.
    pub app_name: Option<String>,
    /// Values for the blueprint's parameter schema.
    #[serde(default)]
    pub parameters: serde_json::Map<String, serde_json::Value>,
    /// One of the blueprint's images; defaults to the first.
    pub image: Option<String>,
    /// Payload files, as text or base64.
    #[serde(default)]
    pub files: Vec<PayloadFile>,
    /// How long the app lives: `<days>d`, `<hours>h` or `forever`.
    /// Defaults to 7 days.
    pub ttl: Option<String>,
    /// Put the app behind basic auth.
    pub basic_auth: Option<BasicAuth>,
    /// The scope to create the app in, one of the scopes you may create in
    /// (listed per blueprint by `list_blueprints`). Omit it when you have a
    /// single such scope, or to use `default`.
    #[serde(default)]
    #[schemars(with = "String")]
    pub scope: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct CreatedApp {
    /// Poll this with `get_task` until its state is `Finished` or `Failed`.
    pub task_id: String,
    pub app_name: String,
    /// The app's public URLs, reachable once the task finished.
    pub urls: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct TaskRef {
    /// The task id returned by `create_app` or `destroy_app`.
    pub task_id: String,
    /// Return the raw output of all commands (last 100 lines) instead of the
    /// task's step summary.
    #[serde(default)]
    pub full_output: bool,
}

/// How many output lines `get_task` returns with `full_output`, from the end.
const TASK_OUTPUT_LINES: usize = 100;
/// How many raw command lines a failed task's summary adds, from the end.
const FAILED_TASK_RAW_LINES: usize = 20;

/// The lines `get_task` returns: Scotty's own step lines, plus the raw tail
/// when the task failed; or, with `full`, the last lines of every stream.
fn task_output(lines: &[OutputLine], failed: bool, full: bool) -> Vec<&OutputLine> {
    if full {
        return lines[lines.len().saturating_sub(TASK_OUTPUT_LINES)..]
            .iter()
            .collect();
    }
    let raw = |l: &&OutputLine| {
        matches!(
            l.stream,
            OutputStreamType::Stdout | OutputStreamType::Stderr
        )
    };
    let raw_tail: Vec<_> = if failed {
        let raw_lines: Vec<_> = lines.iter().filter(raw).collect();
        raw_lines[raw_lines.len().saturating_sub(FAILED_TASK_RAW_LINES)..].to_vec()
    } else {
        Vec::new()
    };
    // Keep the original order, so the raw tail sits next to the step it came from.
    lines
        .iter()
        .filter(|l| !raw(l) || raw_tail.iter().any(|t| t.sequence == l.sequence))
        .collect()
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct TaskInfo {
    pub task_id: String,
    /// `Running`, `Finished` or `Failed`.
    pub state: String,
    pub app_name: Option<String>,
    pub last_exit_code: Option<i32>,
    pub started: String,
    pub finished: Option<String>,
    /// The task's steps (plus the last command output if it failed), or the
    /// last 100 lines of all output with `full_output`.
    pub output: Vec<String>,
    /// Output lines left out of `output`; pass `full_output` to see them.
    pub output_lines_omitted: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct DestroyedApp {
    /// Poll this with `get_task` until the app is gone.
    pub task_id: String,
    pub app_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct AppOperation {
    /// Poll this with `get_task` until its state is `Finished` or `Failed`.
    pub task_id: String,
    pub app_name: String,
}

#[tool_router]
impl ScottyMcp {
    #[tool(
        description = "List the blueprints you may create apps from, with their parameter \
                       schema, allowed images and accepted payload file patterns.",
        annotations(
            title = "List blueprints",
            read_only_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_blueprints(&self, ctx: RequestContext<RoleServer>) -> ToolResult<BlueprintList> {
        let caller = caller(&ctx, McpScope::Read)?;
        let mut blueprints: Vec<_> = apps::blueprints(&self.state, &caller.email, Target::Global)
            .into_iter()
            .filter_map(|(name, bp)| {
                let template = bp.template?;
                Some(BlueprintInfo {
                    scopes: apps::create_scopes(&self.state, &caller.email, Some(&name)),
                    name,
                    title: bp.name,
                    description: bp.description,
                    parameters: template.parameters,
                    images: template.images,
                    payload: template.payload,
                })
            })
            .collect();
        blueprints.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Json(BlueprintList { blueprints }))
    }

    #[tool(
        description = "List the apps you may view, with status and URLs.",
        annotations(title = "List apps", read_only_hint = true, open_world_hint = false)
    )]
    async fn list_apps(&self, ctx: RequestContext<RoleServer>) -> ToolResult<AppList> {
        let caller = caller(&ctx, McpScope::Read)?;
        let apps = apps::list(&self.state, &caller.email).await;
        Ok(Json(AppList {
            apps: apps.apps.iter().map(AppSummary::new).collect(),
        }))
    }

    #[tool(
        description = "Show one app: status, services, URLs, blueprint and scopes.",
        annotations(title = "Get app", read_only_hint = true, open_world_hint = false)
    )]
    async fn get_app(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(AppRef { app_name }): Parameters<AppRef>,
    ) -> ToolResult<AppSummary> {
        let caller = caller(&ctx, McpScope::Read)?;
        let app = apps::get(&self.state, &caller.email, &app_name)
            .await
            .map_err(|e| e.to_string())?;
        Ok(Json(AppSummary::new(&app)))
    }

    #[tool(
        description = "Create an app from a templated blueprint, e.g. publish an HTML page \
                       with blueprint `static-html` and file `html/index.html`. Returns \
                       immediately with a task id to poll with get_task, and the app's URLs.",
        annotations(
            title = "Create app",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn create_app(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(input): Parameters<CreateAppInput>,
    ) -> ToolResult<CreatedApp> {
        let caller = caller(&ctx, McpScope::Create)?;
        let request = create_request(&self.state, &caller.email, input)?;
        let created = apps::create(
            &self.state,
            &caller.email,
            request,
            apps::OnExisting::Refuse,
        )
        .await
        .map_err(|e| e.to_string())?;
        let urls = created
            .app_data
            .settings
            .as_ref()
            .map(|s| expected_urls(&self.state, s))
            .unwrap_or_default();
        Ok(Json(CreatedApp {
            task_id: created.task.id.to_string(),
            app_name: created.app_data.name,
            urls,
        }))
    }

    #[tool(
        description = "Show a task's state (Running, Finished or Failed) and a summary of its \
                       steps; a failed task also shows its last command output. Pass \
                       full_output for the raw output. Poll it after create_app or destroy_app.",
        annotations(title = "Get task", read_only_hint = true, open_world_hint = false)
    )]
    async fn get_task(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(TaskRef {
            task_id,
            full_output,
        }): Parameters<TaskRef>,
    ) -> ToolResult<TaskInfo> {
        let caller = caller(&ctx, McpScope::Read)?;
        let id = uuid::Uuid::parse_str(&task_id)
            .map_err(|_| format!("task_id: {task_id:?} is not a task id"))?;
        let task = apps::task(&self.state, &caller.email, &id)
            .await
            .map_err(|e| e.to_string())?;
        let shown = task_output(&task.output.lines, task.state == State::Failed, full_output);
        Ok(Json(TaskInfo {
            task_id: task.id.to_string(),
            state: wire_name(&task.state),
            app_name: task.app_name.clone(),
            last_exit_code: task.last_exit_code,
            started: task.start_time.to_rfc3339(),
            finished: task.finish_time.map(|t| t.to_rfc3339()),
            output: shown.iter().map(|l| l.content.clone()).collect(),
            output_lines_omitted: task
                .output
                .total_lines_processed
                .saturating_sub(shown.len() as u64),
        }))
    }

    #[tool(
        description = "Destroy an app: stop and remove its containers, volumes and files. \
                       This cannot be undone.",
        annotations(
            title = "Destroy app",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn destroy_app(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(AppRef { app_name }): Parameters<AppRef>,
    ) -> ToolResult<DestroyedApp> {
        let caller = caller(&ctx, McpScope::Destroy)?;
        let destroyed = apps::destroy(&self.state, &caller.email, &app_name)
            .await
            .map_err(|e| e.to_string())?;
        Ok(Json(DestroyedApp {
            task_id: destroyed.task.id.to_string(),
            app_name: destroyed.app_data.name,
        }))
    }

    #[tool(
        description = "Stop an app's containers; its files and volumes stay. Returns a task id \
                       to poll with get_task.",
        annotations(
            title = "Stop app",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn stop_app(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(AppRef { app_name }): Parameters<AppRef>,
    ) -> ToolResult<AppOperation> {
        self.lifecycle(&ctx, &app_name, apps::Lifecycle::Stop).await
    }

    #[tool(
        description = "Start a stopped app. Returns a task id to poll with get_task.",
        annotations(
            title = "Start app",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn start_app(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(AppRef { app_name }): Parameters<AppRef>,
    ) -> ToolResult<AppOperation> {
        self.lifecycle(&ctx, &app_name, apps::Lifecycle::Start)
            .await
    }

    #[tool(
        description = "Rebuild an app: pull its images, rebuild and restart its containers; \
                       files and volumes stay. Returns a task id to poll with get_task.",
        annotations(
            title = "Rebuild app",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn rebuild_app(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(AppRef { app_name }): Parameters<AppRef>,
    ) -> ToolResult<AppOperation> {
        self.lifecycle(&ctx, &app_name, apps::Lifecycle::Rebuild)
            .await
    }
}

impl ScottyMcp {
    async fn lifecycle(
        &self,
        ctx: &RequestContext<RoleServer>,
        app_name: &str,
        op: apps::Lifecycle,
    ) -> ToolResult<AppOperation> {
        let caller = caller(ctx, McpScope::Manage)?;
        let running = apps::lifecycle(&self.state, &caller.email, app_name, op)
            .await
            .map_err(|e| e.to_string())?;
        Ok(Json(AppOperation {
            task_id: running.task.id.to_string(),
            app_name: running.app_data.name,
        }))
    }
}

/// The create request for `input`: only templated blueprints, payload
/// decoded and capped by `api.create_app_max_size`, a generated name
/// when none is given.
fn create_request(
    state: &SharedAppState,
    user_id: &str,
    input: CreateAppInput,
) -> Result<CreateAppRequest, String> {
    let blueprint = state
        .settings
        .apps
        .blueprints
        .get(&input.blueprint)
        .ok_or_else(|| format!("Unknown blueprint {}; see list_blueprints", input.blueprint))?;
    if blueprint.template.is_none() {
        return Err(format!(
            "Blueprint {} needs a compose file; create_app accepts only templated blueprints \
             (see list_blueprints)",
            input.blueprint
        ));
    }

    let limit = state.settings.api.create_app_max_size;
    let mut total = 0usize;
    let mut files = Vec::with_capacity(input.files.len());
    for file in input.files {
        let bytes = match file.encoding {
            Encoding::Utf8 => file.content.into_bytes(),
            Encoding::Base64 => BASE64_STANDARD
                .decode(file.content.trim())
                .map_err(|e| format!("files: {} is not valid base64: {e}", file.path))?,
        };
        total = total.saturating_add(bytes.len());
        if total > limit {
            return Err(format!(
                "files: the payload exceeds the limit of {} (api.create_app_max_size)",
                format_bytes(limit)
            ));
        }
        files.push(File {
            name: file.path,
            content: bytes.into(),
            // Already decoded above; `apps::create` must not decode again.
            compressed: false,
        });
    }

    let app_name = match input.app_name {
        Some(name) => {
            let slug = slugify(&name);
            if slug.is_empty() {
                return Err(format!("app_name: {name:?} is not a valid app name"));
            }
            slug
        }
        None => generated_name(&input.blueprint)?,
    };
    let time_to_live = match input.ttl {
        Some(ttl) => match ttl.parse::<AppTtl>() {
            Err(_) => {
                return Err(format!(
                    "ttl: {ttl:?} is invalid; use <days>d or <hours>h (at least 1), or forever"
                ))
            }
            Ok(ttl) => ttl,
        },
        None => AppSettings::default().time_to_live,
    };
    let scope = match input.scope {
        Some(scope) => scope,
        None => default_scope(apps::create_scopes(state, user_id, Some(&input.blueprint)))?,
    };
    Ok(CreateAppRequest {
        app_name,
        settings: AppSettings {
            app_blueprint: Some(input.blueprint),
            time_to_live,
            basic_auth: input.basic_auth.map(|b| (b.user, b.password)),
            ..AppSettings::default()
        },
        files: FileList { files },
        custom_domains: vec![],
        requested_scopes: vec![scope],
        blueprint_parameters: input.parameters,
        image: input.image,
    })
}

/// The scope for a `create_app` without `scope`: the caller's only create
/// scope, else `default` (also when they have none, so the create check
/// reports the refusal); with several and no `default`, the caller chooses.
pub(crate) fn default_scope(allowed: Vec<String>) -> Result<String, String> {
    const DEFAULT: &str = "default";
    match allowed.as_slice() {
        [only] => Ok(only.clone()),
        [] => Err("you may not create apps in any scope".to_string()),
        _ if allowed.iter().any(|s| s == DEFAULT) => Ok(DEFAULT.to_string()),
        _ => Err(format!("scope: choose one of {}", allowed.join(", "))),
    }
}

/// `<blueprint>-<10 random [a-z0-9]>`: about 51 bits, so artifact URLs
/// can't be guessed.
pub(crate) fn generated_name(blueprint: &str) -> Result<String, String> {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut suffix = String::with_capacity(10);
    while suffix.len() < 10 {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|e| format!("can't generate a name: {e}"))?;
        // Rejection sampling: 252 is the largest multiple of 36 below 256.
        suffix.extend(
            bytes
                .iter()
                .filter(|b| **b < 252)
                .map(|b| ALPHABET[usize::from(*b) % ALPHABET.len()] as char)
                .take(10 - suffix.len()),
        );
    }
    Ok(slugify(format!("{blueprint}-{suffix}")))
}

/// The URLs the app will be served at, from its settings after the
/// blueprint was applied (the containers don't exist yet).
fn expected_urls(state: &SharedAppState, settings: &AppSettings) -> Vec<String> {
    let tls = match state.settings.load_balancer_type {
        LoadBalancerType::Traefik => state.settings.traefik.use_tls,
        LoadBalancerType::HaproxyConfig => state.settings.haproxy.use_tls,
    };
    let scheme = if tls { "https" } else { "http" };
    settings
        .public_services
        .iter()
        .flat_map(|s| s.get_domains(&settings.domain))
        .map(|domain| format!("{scheme}://{domain}"))
        .collect()
}

#[tool_handler(
    name = "scotty",
    instructions = "Scotty runs Docker Compose apps. Create apps from blueprints \
                    (list_blueprints, create_app), poll the returned task with get_task, \
                    inspect apps with list_apps/get_app, stop, start or rebuild them with \
                    stop_app/start_app/rebuild_app and remove them with destroy_app."
)]
impl ServerHandler for ScottyMcp {
    /// The tools, with `create_app`'s `scope` offering the caller's create
    /// scopes as an enum, so clients can show them as choices. The list is
    /// per caller, hence not cacheable across users.
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let mut tools = Self::tool_router().list_all();
        if let Ok(caller) = caller(&ctx, McpScope::Create) {
            let scopes = apps::create_scopes(&self.state, &caller.email, None);
            if let Some(tool) = tools.iter_mut().find(|t| t.name == "create_app") {
                offer_scopes(tool, &scopes);
            }
        }
        let cache_hints = ctx
            .protocol_version()
            .is_some_and(|version| version >= ProtocolVersion::V_2026_07_28);
        Ok(ListToolsResult {
            result_type: Some(ResultType::COMPLETE),
            tools,
            meta: None,
            next_cursor: None,
            ttl_ms: cache_hints.then_some(0),
            cache_scope: cache_hints.then_some(CacheScope::Private),
        })
    }
}

/// Restrict `create_app`'s `scope` input to `scopes` (left open when empty:
/// an empty enum would make the tool uncallable instead of explaining why).
fn offer_scopes(tool: &mut Tool, scopes: &[String]) {
    if scopes.is_empty() {
        return;
    }
    let mut schema = (*tool.input_schema).clone();
    if let Some(scope) = schema
        .get_mut("properties")
        .and_then(|p| p.get_mut("scope"))
        .and_then(serde_json::Value::as_object_mut)
    {
        scope.insert("enum".into(), scopes.into());
        tool.input_schema = Arc::new(schema);
    }
}

/// The caller of this tool call, put into the request by the `/mcp` auth
/// middleware, if its token carries `scope`.
fn caller(ctx: &RequestContext<RoleServer>, scope: McpScope) -> Result<McpPrincipal, String> {
    let principal = ctx
        .extensions
        .get::<axum::http::request::Parts>()
        .and_then(|parts| parts.extensions.get::<McpPrincipal>())
        .cloned()
        // Only if the route were mounted without the auth middleware.
        .ok_or_else(|| "not authenticated".to_string())?;
    principal.require_scope(scope)?;
    Ok(principal)
}

/// The rmcp service behind `/mcp`, stateless: no
/// `Mcp-Session-Id`, so restarts and several instances need no affinity.
pub fn service(state: &SharedAppState) -> StreamableHttpService<ScottyMcp, NeverSessionManager> {
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_sse_keep_alive(None)
        // rmcp's Host allowlist guards unauthenticated local servers against
        // DNS rebinding. Every `/mcp` request here needs a bearer token,
        // which a rebinding page cannot attach, and Scotty is reached under
        // whatever host its proxy forwards. If `/mcp` ever accepts cookie or
        // session auth, re-enable the allowlist: rebinding matters again.
        .disable_allowed_hosts()
        // Inline payloads arrive as JSON text or base64; the create size
        // limit is checked on the decoded bytes in `create_app`, this only
        // bounds the request. JSON-escaped text (`\uXXXX`, surrogate pairs)
        // takes up to 3x its UTF-8 size, base64 4/3x.
        .with_max_request_body_bytes(
            state
                .settings
                .api
                .create_app_max_size
                .saturating_mul(3)
                .saturating_add(64 * 1024),
        );
    let state = state.clone();
    StreamableHttpService::new(
        move || {
            Ok(ScottyMcp {
                state: state.clone(),
            })
        },
        Arc::new(NeverSessionManager::default()),
        config,
    )
}

#[cfg(test)]
mod task_output_tests {
    use super::*;

    fn lines() -> Vec<OutputLine> {
        let mut seq = 0;
        let mut line = |stream, text: &str| {
            seq += 1;
            OutputLine::new(stream, text.into(), seq)
        };
        let mut out = vec![line(OutputStreamType::Status, "Starting app 'a'")];
        out.extend(
            (0..30).map(|i| line(OutputStreamType::Stdout, &format!("layer {i} Downloading"))),
        );
        out.push(line(
            OutputStreamType::Status,
            "Completed: docker-compose pull",
        ));
        out.push(line(OutputStreamType::Stderr, "boom"));
        out.push(line(OutputStreamType::StatusError, "Failed"));
        out
    }

    fn texts(shown: Vec<&OutputLine>) -> Vec<&str> {
        shown.iter().map(|l| l.content.as_str()).collect()
    }

    #[test]
    fn a_finished_task_shows_only_its_steps() {
        let lines = lines();
        assert_eq!(
            texts(task_output(&lines, false, false)),
            [
                "Starting app 'a'",
                "Completed: docker-compose pull",
                "Failed"
            ]
        );
    }

    #[test]
    fn a_failed_task_adds_the_last_raw_lines_in_order() {
        let lines = lines();
        let shown = texts(task_output(&lines, true, false));
        assert_eq!(shown.len(), 3 + FAILED_TASK_RAW_LINES);
        assert_eq!(shown[0], "Starting app 'a'");
        assert_eq!(shown[1], "layer 11 Downloading");
        assert_eq!(
            &shown[shown.len() - 3..],
            ["Completed: docker-compose pull", "boom", "Failed"]
        );
    }

    #[test]
    fn short_and_empty_outputs() {
        assert!(task_output(&[], true, false).is_empty());
        let lines = [
            OutputLine::new(OutputStreamType::Status, "Starting".into(), 1),
            OutputLine::stdout("pulled".into(), 2),
            OutputLine::stderr("boom".into(), 3),
            OutputLine::new(OutputStreamType::StatusError, "Failed".into(), 4),
        ];
        assert_eq!(
            texts(task_output(&lines, true, false)),
            ["Starting", "pulled", "boom", "Failed"]
        );
    }

    #[test]
    fn full_output_is_the_raw_tail() {
        let lines: Vec<_> = (0..150)
            .map(|i| OutputLine::stdout(format!("line {i}"), i))
            .collect();
        let shown = texts(task_output(&lines, false, true));
        assert_eq!(shown.len(), TASK_OUTPUT_LINES);
        assert_eq!(shown[0], "line 50");
    }
}
