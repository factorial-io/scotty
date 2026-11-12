//! The MCP authorization server's HTTP side (design D2, D6): grantry's
//! protocol routes (metadata, authorize, token, register), RFC 9728
//! protected resource metadata, and the `/mcp` token check with the
//! scope-plus-policy helper the `mcp-server` tools use.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post_service},
    Json, Router,
};
use grantry::metadata::{AuthorizationServerMetadata, ProtectedResourceMetadata};
use grantry::routes::Routes;
use scotty_core::authorization::Permission;
use tracing::{debug, error};
use uuid::Uuid;

use super::server::{mcp_resource, McpServer};
use crate::api::auth_core::authorize_bearer_user;
use crate::api::rate_limiting::create_register_limiter;
use crate::app_state::{AppState, SharedAppState};
use crate::services::authorization::decision::Target;

pub const AUTHORIZE_PATH: &str = "/oauth/mcp/authorize";
pub const TOKEN_PATH: &str = "/oauth/mcp/token";
pub const REGISTER_PATH: &str = "/oauth/mcp/register";
pub const RESOURCE_METADATA_PATH: &str = "/.well-known/oauth-protected-resource/mcp";
const RESOURCE_METADATA_ROOT: &str = "/.well-known/oauth-protected-resource";
pub const MCP_PATH: &str = "/mcp";

/// `/mcp`, in every auth mode, plus the authorization server's routes when
/// it is configured (oauth mode with a `jwt_secret`).
pub fn router(state: &SharedAppState) -> Router {
    let mcp = Router::new()
        .route_service(MCP_PATH, crate::api::mcp::service(state))
        .route_layer(middleware::from_fn_with_state(state.clone(), mcp_auth));
    match authorization_server(state) {
        Some(protocol) => protocol.merge(mcp),
        None => mcp,
    }
}

/// The authorization server's routes, or `None` when it is not configured.
fn authorization_server(state: &SharedAppState) -> Option<Router> {
    let server = state.mcp_server.clone()?;
    let base = state.settings.api.public_base_url();
    let frontend = super::frontend_base(state);
    let consent_url: Arc<dyn Fn(Uuid) -> String + Send + Sync> =
        Arc::new(move |id| format!("{frontend}/oauth/consent/{id}"));
    let routes = |register_path: Option<&str>| Routes {
        metadata: AuthorizationServerMetadata::new(
            &base,
            AUTHORIZE_PATH,
            TOKEN_PATH,
            Some(REGISTER_PATH),
            &server.config.scopes.known,
        ),
        authorize_path: AUTHORIZE_PATH.into(),
        token_path: TOKEN_PATH.into(),
        register_path: register_path.map(str::to_owned),
        consent_url: consent_url.clone(),
        other_grant: None,
    };

    // grantry mounts all its routes in one router; registration alone gets
    // the per-address limiter, so it is served from a second instance that
    // only receives the register path.
    let protocol = grantry::routes::router(server.clone(), routes(None));
    let registering = grantry::routes::router(server.clone(), routes(Some(REGISTER_PATH)));
    // Always on (spec: registration is rate limited per client address),
    // whatever `api.rate_limiting.enabled` says.
    let register = post_service(registering).layer(create_register_limiter(Some(
        &state.settings.api.rate_limiting.oauth,
    )));

    let resource_metadata = Json(ProtectedResourceMetadata::new(
        &mcp_resource(&base),
        &base,
        &server.config.scopes.known,
        Some("Scotty"),
    ));
    let metadata = move || {
        let doc = resource_metadata.clone();
        async move { ([(header::CACHE_CONTROL, "public, max-age=300")], doc) }
    };

    Some(
        protocol
            .route(REGISTER_PATH, register)
            .route(RESOURCE_METADATA_PATH, get(metadata.clone()))
            .route(RESOURCE_METADATA_ROOT, get(metadata)),
    )
}

/// Who an MCP request acts for: the grant's user and the approved scopes,
/// or a configured bearer token's service account with every scope (its
/// policy permissions are its only limit).
#[derive(Debug, Clone)]
pub struct McpPrincipal {
    /// The user id for authorization: an email, or `identifier:<name>`.
    pub email: String,
    pub scopes: Vec<String>,
}

/// An MCP scope a tool requires (D6).
#[allow(dead_code)] // read by the `mcp-server` tools
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpScope {
    /// View apps, blueprints and tasks; read logs.
    Read,
    /// Create apps.
    Create,
    /// Destroy apps.
    Destroy,
}

#[allow(dead_code)] // read by the `mcp-server` tools
impl McpScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Create => "create",
            Self::Destroy => "destroy",
        }
    }

    /// Whether this scope may stand for `permission` at all.
    fn covers(self, permission: &Permission) -> bool {
        matches!(
            (self, permission),
            (Self::Read, Permission::View | Permission::Logs)
                | (Self::Create, Permission::Create)
                | (Self::Destroy, Permission::Destroy)
        )
    }
}

/// Where a permission is checked: on an app, or in any of some scopes (app
/// creation, where the app does not exist yet).
#[allow(dead_code)] // read by the `mcp-server` tools
#[derive(Debug, Clone, Copy)]
pub enum McpTarget<'a> {
    App(&'a str),
    Scopes(&'a [String]),
}

#[allow(dead_code)] // read by the `mcp-server` tools
impl McpPrincipal {
    /// D6: allowed only if the token carries `scope`, `scope` stands for
    /// `permission`, and the policy gives the user `permission` on `target`.
    pub async fn allow(
        &self,
        state: &AppState,
        scope: McpScope,
        permission: Permission,
        target: McpTarget<'_>,
    ) -> bool {
        if !scope.covers(&permission) || !self.scopes.iter().any(|s| s == scope.as_str()) {
            return false;
        }
        let target = match target {
            McpTarget::App(app) => Target::App(app),
            McpTarget::Scopes(scopes) => Target::AnyOf(scopes),
        };
        state
            .auth_service
            .decide(&self.email, permission.into(), target)
            .is_allowed()
    }
}

/// `/mcp` authentication: a configured bearer token, or an mcp access token
/// (audience `{base}/mcp`, kind `mcp`) whose grant is still active. Anything
/// else, session and identity-provider tokens included, is 401 with the RFC
/// 9728 §5.1 challenge pointing at the protected resource metadata.
pub async fn mcp_auth(
    State(state): State<SharedAppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map(|(_, token)| token.trim().to_owned());
    let Some(token) = token else {
        return unauthorized(&state, false);
    };
    if let Some(user) = authorize_bearer_user(state.clone(), &token, false).await {
        req.extensions_mut().insert(McpPrincipal {
            email: user.email,
            scopes: [McpScope::Read, McpScope::Create, McpScope::Destroy]
                .map(|s| s.as_str().to_owned())
                .into(),
        });
        return next.run(req).await;
    }
    let Some(server) = state.mcp_server.as_ref() else {
        return unauthorized(&state, true);
    };
    match principal(server, &token).await {
        Ok(Some(principal)) => {
            req.extensions_mut().insert(principal);
            next.run(req).await
        }
        Ok(None) => unauthorized(&state, true),
        Err(e) => {
            error!("Failed to load the mcp token's grant: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn principal(
    server: &McpServer,
    token: &str,
) -> Result<Option<McpPrincipal>, grantry::BackendError> {
    match server.verify_active(token).await {
        Ok((grant, _claims)) => Ok(Some(McpPrincipal {
            scopes: grant.scope.split_whitespace().map(str::to_owned).collect(),
            email: grant.subject,
        })),
        Err(grantry::ActiveError::Backend(e)) => Err(e),
        Err(e) => {
            debug!("mcp token rejected: {}", e);
            Ok(None)
        }
    }
}

fn unauthorized(state: &AppState, token_sent: bool) -> Response {
    let metadata_url = format!(
        "{}{RESOURCE_METADATA_PATH}",
        state.settings.api.public_base_url()
    );
    (
        StatusCode::UNAUTHORIZED,
        [(
            header::WWW_AUTHENTICATE,
            ProtectedResourceMetadata::challenge(&metadata_url, token_sent),
        )],
    )
        .into_response()
}
