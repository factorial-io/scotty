//! The MCP endpoint at `/mcp`: a tools-only server over stateless
//! Streamable HTTP (rmcp). Every tool acts as the authenticated caller
//! ([`McpPrincipal`], set by [`crate::oauth::mcp::mcp_auth`]): it checks the
//! token's scope here, then calls the same `services::apps` function as the
//! REST API, which checks the caller's permissions.
//!
//! Results never carry environment values, generated parameters, basic
//! auth credentials or tokens: apps are reported as [`AppSummary`].

use std::sync::Arc;

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
use scotty_core::apps::app_data::{AppData, AppSettings};
use serde::{Deserialize, Serialize};

use crate::app_state::SharedAppState;
use crate::oauth::mcp::{McpPrincipal, McpScope};
use crate::services::apps;

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
        let mut blueprints: Vec<_> = apps::blueprints(&self.state, &caller.email)
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
        // bounds the request.
        .with_max_request_body_bytes(state.settings.api.create_app_max_size.saturating_mul(2));
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
