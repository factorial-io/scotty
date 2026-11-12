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
use scotty_core::settings::loadbalancer::LoadBalancerType;
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
                    status: format!("{:?}", s.status),
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

/// `create_app` input (design D5).
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct CreateAppInput {
    /// A templated blueprint from `list_blueprints`.
    pub blueprint: String,
    /// The app name. Omit it to get an unguessable generated name
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
}

/// How many output lines `get_task` returns, from the end.
const TASK_OUTPUT_LINES: usize = 100;

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
    /// The last output lines (at most 100).
    pub output: Vec<String>,
    /// Lines left out at the start of `output`.
    pub output_lines_omitted: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct DestroyedApp {
    /// Poll this with `get_task` until the app is gone.
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
        // Only what create_app can use: it creates in the MCP scopes.
        let scopes = mcp_scopes();
        let mut blueprints: Vec<_> =
            apps::blueprints(&self.state, &caller.email, Target::AnyOf(&scopes))
                .into_iter()
                .filter_map(|(name, bp)| {
                    let template = bp.template?;
                    Some(BlueprintInfo {
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
        let request = create_request(&self.state, input)?;
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
        description = "Show a task's state (Running, Finished or Failed) and its last output \
                       lines. Poll it after create_app or destroy_app.",
        annotations(title = "Get task", read_only_hint = true, open_world_hint = false)
    )]
    async fn get_task(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(TaskRef { task_id }): Parameters<TaskRef>,
    ) -> ToolResult<TaskInfo> {
        let caller = caller(&ctx, McpScope::Read)?;
        let id = uuid::Uuid::parse_str(&task_id)
            .map_err(|_| format!("task_id: {task_id:?} is not a task id"))?;
        let task = apps::task(&self.state, &caller.email, &id)
            .await
            .map_err(|e| e.to_string())?;
        let lines = &task.output.lines;
        let shown = &lines[lines.len().saturating_sub(TASK_OUTPUT_LINES)..];
        Ok(Json(TaskInfo {
            task_id: task.id.to_string(),
            state: format!("{:?}", task.state),
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
}

/// The create request for `input` (D5): only templated blueprints, payload
/// decoded and capped by `api.create_app_max_size`, a generated name (D6)
/// when none is given.
fn create_request(
    state: &SharedAppState,
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
            // Zero would have the app stopped right after it started.
            Ok(AppTtl::Days(0) | AppTtl::Hours(0)) | Err(_) => {
                return Err(format!(
                    "ttl: {ttl:?} is invalid; use <days>d or <hours>h (at least 1), or forever"
                ))
            }
            Ok(ttl) => ttl,
        },
        None => AppSettings::default().time_to_live,
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
        requested_scopes: mcp_scopes(),
        blueprint_parameters: input.parameters,
        image: input.image,
    })
}

/// The scopes MCP apps are created in. D5 has no scopes field, so they land
/// in `default`, like a REST create without `requested_scopes`.
// ponytail: add a `scope` input to create_app if users without create
// rights in `default` need MCP.
fn mcp_scopes() -> Vec<String> {
    AppSettings::default().scopes
}

/// `<blueprint>-<10 random [a-z0-9]>` (D6): about 51 bits, so artifact URLs
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
                    inspect apps with list_apps/get_app and remove them with destroy_app."
)]
impl ServerHandler for ScottyMcp {}

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

/// The rmcp service behind `/mcp`, stateless (design D2): no
/// `Mcp-Session-Id`, so restarts and several instances need no affinity.
pub fn service(state: &SharedAppState) -> StreamableHttpService<ScottyMcp, NeverSessionManager> {
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_sse_keep_alive(None)
        // rmcp's Host allowlist guards unauthenticated local servers against
        // DNS rebinding. Every `/mcp` request here needs a bearer token,
        // which a rebinding page cannot attach, and Scotty is reached under
        // whatever host its proxy forwards.
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
