//! The MCP authorization server's HTTP side: grantry's
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
use tracing::{debug, error};
use uuid::Uuid;

use super::server::{mcp_resource, McpServer};
use crate::api::auth_core::authorize_bearer_user;
use crate::api::error::AppError;
use crate::api::rate_limiting::middleware::RateLimitMetricsLayer;
use crate::api::rate_limiting::{create_authenticated_limiter, create_register_limiter};
use crate::app_state::{AppState, SharedAppState};

pub const AUTHORIZE_PATH: &str = "/oauth/mcp/authorize";
pub const TOKEN_PATH: &str = "/oauth/mcp/token";
pub const REGISTER_PATH: &str = "/oauth/mcp/register";
pub const RESOURCE_METADATA_PATH: &str = "/.well-known/oauth-protected-resource/mcp";
const RESOURCE_METADATA_ROOT: &str = "/.well-known/oauth-protected-resource";
pub const MCP_PATH: &str = "/mcp";

/// `/mcp`, in every auth mode, plus the authorization server's routes when
/// it is configured (oauth mode with a `jwt_secret`).
pub fn router(state: &SharedAppState) -> Router {
    let mut mcp = Router::new()
        .route_service(MCP_PATH, crate::api::mcp::service(state))
        .route_layer(middleware::from_fn_with_state(state.clone(), mcp_auth));
    // The authenticated tier's limit, keyed by bearer token, as for REST.
    let limits = &state.settings.api.rate_limiting;
    if limits.enabled && limits.authenticated.is_enabled() {
        mcp = mcp
            .layer(create_authenticated_limiter(&limits.authenticated))
            .layer(RateLimitMetricsLayer::new("authenticated"));
    }
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
/// or a configured bearer token's service account. A bearer token passes
/// every tool's MCP scope check (`read`, `create`, `destroy`); its
/// `identifier:<name>` assignment decides which Scotty scopes and apps it
/// reaches.
#[derive(Debug, Clone)]
pub struct McpPrincipal {
    /// The user id for authorization: an email, or `identifier:<name>`.
    pub email: String,
    pub scopes: Vec<String>,
}

/// An MCP scope a tool requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpScope {
    /// View apps, blueprints and tasks; read logs.
    Read,
    /// Create apps.
    Create,
    /// Destroy apps.
    Destroy,
}

impl McpScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Create => "create",
            Self::Destroy => "destroy",
        }
    }
}

impl McpPrincipal {
    /// The per-tool scope check: `Err` names the missing scope, so the
    /// client can tell the user which permission to grant when reconnecting.
    pub fn require_scope(&self, scope: McpScope) -> Result<(), String> {
        if self.scopes.iter().any(|s| s == scope.as_str()) {
            Ok(())
        } else {
            Err(format!(
                "Access denied: this connection lacks the `{}` scope; reconnect and approve it",
                scope.as_str()
            ))
        }
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
    // As the REST authorization middleware: without assignments nobody is
    // authorized (and the app list would otherwise show every app).
    if !state.auth_service.is_enabled().await {
        return AppError::AuthorizationNotConfigured.into_response();
    }
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
    // Without the authorization server the metadata document 404s (see
    // `static_files::serve_embedded_file`), so don't
    // point clients at an OAuth flow that doesn't exist.
    let challenge = match (&state.mcp_server, token_sent) {
        (Some(_), _) => ProtectedResourceMetadata::challenge(
            &format!(
                "{}{RESOURCE_METADATA_PATH}",
                state.settings.api.public_base_url()
            ),
            token_sent,
        ),
        (None, true) => r#"Bearer error="invalid_token""#.to_string(),
        (None, false) => "Bearer".to_string(),
    };
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, challenge)],
    )
        .into_response()
}
