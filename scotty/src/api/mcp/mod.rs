//! The MCP endpoint at `/mcp`: a tools-only server over stateless
//! Streamable HTTP (rmcp). Every tool acts as the authenticated caller
//! ([`McpPrincipal`], set by [`crate::oauth::mcp::mcp_auth`]) with the same
//! authorization as the REST API, narrowed by the token's scopes.

use std::sync::Arc;

use rmcp::{
    service::RequestContext,
    tool_handler, tool_router,
    transport::streamable_http_server::{
        session::never::NeverSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    },
    RoleServer, ServerHandler,
};

use crate::app_state::SharedAppState;
use crate::oauth::mcp::McpPrincipal;

#[derive(Clone)]
pub struct ScottyMcp {
    #[expect(dead_code, reason = "read by the tools")]
    state: SharedAppState,
}

#[tool_router(allow_empty)]
impl ScottyMcp {}

#[tool_handler(
    name = "scotty",
    instructions = "Scotty runs Docker Compose apps. Create apps from blueprints \
                    (list_blueprints, create_app), poll the returned task with get_task, \
                    inspect apps with list_apps/get_app and remove them with destroy_app."
)]
impl ServerHandler for ScottyMcp {}

/// The caller of this tool call, put into the request by the `/mcp` auth
/// middleware. Missing only if the route were mounted without it.
#[expect(dead_code, reason = "read by the tools")]
fn principal(ctx: &RequestContext<RoleServer>) -> Result<McpPrincipal, String> {
    ctx.extensions
        .get::<axum::http::request::Parts>()
        .and_then(|parts| parts.extensions.get::<McpPrincipal>())
        .cloned()
        .ok_or_else(|| "not authenticated".to_string())
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
