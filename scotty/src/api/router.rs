use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::Router;
use scotty_core::apps::app_data::AppData;
use scotty_core::apps::app_data::AppSettings;
use scotty_core::apps::app_data::AppTtl;
use scotty_core::apps::app_data::ContainerState;
use scotty_core::apps::app_data::ServicePortMapping;
use scotty_core::apps::app_data::{AppStatus, LoadBalancerConnectivity};
use scotty_core::apps::create_app_request::CreateAppRequest;
use scotty_core::apps::file_list::File;
use scotty_core::apps::file_list::FileList;
use scotty_core::apps::shared_app_list::AppDataVec;
use scotty_core::notification_types::AddNotificationRequest;
use scotty_core::notification_types::GitlabContext;
use scotty_core::notification_types::MattermostContext;
use scotty_core::notification_types::NotificationReceiver;
use scotty_core::notification_types::WebhookContext;
use scotty_core::settings::custom_action::{
    ActionStatus, CreateCustomActionRequest, CustomAction, CustomActionList, ReviewActionRequest,
};
use scotty_core::tasks::running_app_context::RunningAppContext;

use utoipa::openapi::security::SecurityScheme;
use utoipa::Modify;
use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

use crate::api::rest::handlers::apps::create::__path_create_app_handler;
use crate::api::rest::handlers::apps::custom_action::__path_run_custom_action_handler;
use crate::api::rest::handlers::apps::custom_action_management::{
    __path_create_custom_action_handler, __path_delete_custom_action_handler,
    __path_get_custom_action_handler, __path_list_custom_actions_handler,
};
use crate::api::rest::handlers::apps::list::__path_list_apps_handler;
use crate::api::rest::handlers::apps::list::list_apps_handler;
use crate::api::rest::handlers::apps::notify::__path_add_notification_handler;
use crate::api::rest::handlers::apps::notify::__path_remove_notification_handler;
use crate::api::rest::handlers::apps::run::__path_adopt_app_handler;
use crate::api::rest::handlers::apps::run::__path_destroy_app_handler;
use crate::api::rest::handlers::apps::run::__path_info_app_handler;
use crate::api::rest::handlers::apps::run::__path_purge_app_handler;
use crate::api::rest::handlers::apps::run::__path_rebuild_app_handler;
use crate::api::rest::handlers::apps::run::__path_run_app_handler;
use crate::api::rest::handlers::apps::run::__path_stop_app_handler;
use crate::api::rest::handlers::health::__path_health_checker_handler;
use crate::api::rest::handlers::info::__path_info_handler;
use crate::api::rest::handlers::login::__path_login_handler;
use crate::api::rest::handlers::login::__path_logout_handler;
use crate::api::rest::handlers::login::__path_validate_token_handler;
use crate::oauth::connections::{
    __path_list_connections_handler, __path_revoke_connection_handler, list_connections_handler,
    revoke_connection_handler, Connection,
};
use crate::oauth::consent::{
    __path_approve_consent_handler, __path_deny_consent_handler, __path_get_consent_handler,
    approve_consent_handler, deny_consent_handler, get_consent_handler, ConsentApproval,
    ConsentInfo, ConsentRedirect,
};
use crate::oauth::device::{
    __path_approve_device_handler, __path_deny_device_handler, __path_get_device_handler,
    approve_device_handler, deny_device_handler, device_token, get_device_handler,
    start_device_flow, DeviceDecision, DeviceInfo,
};
use crate::oauth::handlers::{
    exchange_session_for_token, handle_oauth_callback, start_authorization_flow,
};
use crate::oauth::handlers::{CallbackQuery, DeviceFlowResponse, TokenResponse};
use scotty_core::api::{OAuthConfig, ServerInfo};
use scotty_core::settings::api_server::AuthMode;

use super::rest::handlers::landing::{is_scotty_request, landing_or_frontend_handler};
use crate::api::rest::handlers::admin::assignments::{
    __path_create_assignment_handler, __path_list_assignments_handler,
    __path_remove_assignment_handler,
};
use crate::api::rest::handlers::admin::custom_actions::{
    __path_approve_action_handler, __path_get_action_details_handler,
    __path_list_pending_actions_handler, __path_reject_action_handler,
    __path_revoke_action_handler, PendingActionInfo, PendingActionsResponse,
};
use crate::api::rest::handlers::admin::permissions::{
    __path_get_user_permissions_handler, __path_list_available_permissions_handler,
    __path_test_permission_handler,
};
use crate::api::rest::handlers::admin::roles::{
    __path_create_role_handler, __path_list_roles_handler,
};
use crate::api::rest::handlers::admin::scopes::{
    __path_create_scope_handler, __path_list_scopes_handler,
};
use crate::api::rest::handlers::blueprints::__path_blueprints_handler;
use crate::api::rest::handlers::files::{
    __path_download_files_handler, __path_upload_files_handler, download_files_handler,
    upload_files_handler,
};
use crate::api::rest::handlers::health::health_checker_handler;
use crate::api::rest::handlers::scopes::list::__path_list_user_scopes_handler;
use crate::api::rest::handlers::tasks::__path_task_detail_handler;
use crate::api::rest::handlers::tasks::__path_task_list_handler;
use crate::api::rest::handlers::tasks::TaskList;
use crate::api::websocket::client::ws_handler;
use crate::app_state::SharedAppState;
use crate::docker::loadbalancer::ROBOTS_TXT;
use scotty_core::tasks::task_details::TaskDetails;

use super::basic_auth::auth;
use super::middleware::authorization::{authorization_middleware, require_permission};
use super::rate_limiting::{
    create_authenticated_limiter, create_oauth_limiter, create_public_auth_limiter,
};
use super::rest::handlers::admin::assignments::{
    create_assignment_handler, list_assignments_handler, remove_assignment_handler,
};
use super::rest::handlers::admin::custom_actions::{
    approve_action_handler, get_action_details_handler, list_pending_actions_handler,
    reject_action_handler, revoke_action_handler,
};
use super::rest::handlers::admin::permissions::{
    get_user_permissions_handler, list_available_permissions_handler, test_permission_handler,
};
use super::rest::handlers::admin::roles::{create_role_handler, list_roles_handler};
use super::rest::handlers::admin::scopes::{create_scope_handler, list_scopes_handler};
use super::rest::handlers::apps::create::create_app_handler;
use super::rest::handlers::apps::custom_action::run_custom_action_handler;
use super::rest::handlers::apps::custom_action_management::{
    create_custom_action_handler, delete_custom_action_handler, get_custom_action_handler,
    list_custom_actions_handler,
};
use super::rest::handlers::apps::notify::add_notification_handler;
use super::rest::handlers::apps::notify::remove_notification_handler;
use super::rest::handlers::apps::run::adopt_app_handler;
use super::rest::handlers::apps::run::destroy_app_handler;
use super::rest::handlers::apps::run::info_app_handler;
use super::rest::handlers::apps::run::purge_app_handler;
use super::rest::handlers::apps::run::rebuild_app_handler;
use super::rest::handlers::apps::run::run_app_handler;
use super::rest::handlers::apps::run::stop_app_handler;
use super::rest::handlers::blueprints::blueprints_handler;
use super::rest::handlers::info::info_handler;
use super::rest::handlers::login::login_handler;
use super::rest::handlers::login::logout_handler;
use super::rest::handlers::login::validate_token_handler;
use super::rest::handlers::scopes::list::{
    list_user_scopes_handler, ScopeInfo, UserScopesResponse,
};
use super::rest::handlers::tasks::task_detail_handler;
use super::rest::handlers::tasks::task_list_handler;
use crate::services::authorization::types::Assignment;
use crate::services::authorization::Permission;
use scotty_core::admin::{
    AssignmentInfo, AssignmentsListResponse, AvailablePermissionsResponse, CreateAssignmentRequest,
    CreateAssignmentResponse, CreateRoleRequest, CreateRoleResponse, CreateScopeRequest,
    CreateScopeResponse, RemoveAssignmentRequest, RemoveAssignmentResponse, RoleInfo,
    RolesListResponse, ScopeInfo as AdminScopeInfo, ScopesListResponse, TestPermissionRequest,
    TestPermissionResponse, UserPermissionsResponse,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        health_checker_handler,
        list_apps_handler,
        run_app_handler,
        stop_app_handler,
        purge_app_handler,
        info_app_handler,
        task_detail_handler,
        rebuild_app_handler,
        create_app_handler,
        task_list_handler,
        destroy_app_handler,
        validate_token_handler,
        logout_handler,
        get_device_handler,
        approve_device_handler,
        deny_device_handler,
        get_consent_handler,
        approve_consent_handler,
        deny_consent_handler,
        list_connections_handler,
        revoke_connection_handler,
        login_handler,
        info_handler,
        blueprints_handler,
        list_user_scopes_handler,
        add_notification_handler,
        remove_notification_handler,
        adopt_app_handler,
        run_custom_action_handler,
        // Custom action management endpoints
        create_custom_action_handler,
        list_custom_actions_handler,
        get_custom_action_handler,
        delete_custom_action_handler,
        // Admin endpoints
        list_scopes_handler,
        create_scope_handler,
        list_roles_handler,
        create_role_handler,
        list_assignments_handler,
        create_assignment_handler,
        remove_assignment_handler,
        test_permission_handler,
        get_user_permissions_handler,
        list_available_permissions_handler,
        // Admin custom action endpoints
        list_pending_actions_handler,
        get_action_details_handler,
        approve_action_handler,
        reject_action_handler,
        revoke_action_handler,
        download_files_handler,
        upload_files_handler,
    ),
    components(
        schemas(
            GitlabContext, WebhookContext, MattermostContext, NotificationReceiver,
            AddNotificationRequest, TaskList, File, FileList, CreateAppRequest,
            AppData, AppDataVec, TaskDetails, ContainerState, AppSettings,
            AppStatus, AppTtl, ServicePortMapping, RunningAppContext, LoadBalancerConnectivity,
            OAuthConfig, ServerInfo, AuthMode, DeviceFlowResponse, TokenResponse, CallbackQuery,
            DeviceInfo, DeviceDecision,
            ConsentInfo, ConsentApproval, ConsentRedirect, Connection,
            ScopeInfo, UserScopesResponse,
            // Admin API schemas
            AdminScopeInfo, ScopesListResponse, CreateScopeRequest, CreateScopeResponse,
            RoleInfo, RolesListResponse, CreateRoleRequest, CreateRoleResponse,
            AssignmentInfo, AssignmentsListResponse, CreateAssignmentRequest, CreateAssignmentResponse,
            RemoveAssignmentRequest, RemoveAssignmentResponse, Assignment,
            TestPermissionRequest, TestPermissionResponse, UserPermissionsResponse, AvailablePermissionsResponse,
            // Custom action schemas
            CustomAction, ActionStatus, CreateCustomActionRequest, CustomActionList, ReviewActionRequest,
            PendingActionInfo, PendingActionsResponse
        )
    ),
    tags(
        (name = "scotty-service", description = "scotty api")
    ),
    modifiers(&SecurityAddon)

)]
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap(); // we can unwrap safely since there already is components registered.
        components.add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(utoipa::openapi::security::Http::new(
                utoipa::openapi::security::HttpAuthScheme::Bearer,
            )),
        )
    }
}

struct ApiDoc;

impl utoipa::OpenApi for ApiDoc {
    fn openapi() -> utoipa::openapi::OpenApi {
        SecurityAddon::openapi()
    }
}
pub struct ApiRoutes;

impl ApiRoutes {
    pub fn create(state: SharedAppState) -> Router {
        let api = ApiDoc::openapi();
        let rate_limit_config = &state.settings.api.rate_limiting;

        // Build authenticated router with conditional rate limiting
        let mut authenticated_router = Router::new()
            // Routes that require specific permissions
            .route("/api/v1/authenticated/apps/list", get(list_apps_handler))
            .route(
                "/api/v1/authenticated/apps/run/{app_id}",
                get(run_app_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::Manage),
                )),
            )
            .route(
                "/api/v1/authenticated/apps/stop/{app_id}",
                get(stop_app_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::Manage),
                )),
            )
            .route(
                "/api/v1/authenticated/apps/purge/{app_id}",
                get(purge_app_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::Manage),
                )),
            )
            .route(
                "/api/v1/authenticated/apps/rebuild/{app_id}",
                get(rebuild_app_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::Manage),
                )),
            )
            .route(
                "/api/v1/authenticated/apps/info/{app_id}",
                get(info_app_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::View),
                )),
            )
            .route(
                "/api/v1/authenticated/apps/destroy/{app_id}",
                get(destroy_app_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::Destroy),
                )),
            )
            .route(
                "/api/v1/authenticated/apps/adopt/{app_id}",
                get(adopt_app_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::Create),
                )),
            )
            .route(
                "/api/v1/authenticated/apps/create",
                post(create_app_handler).layer(DefaultBodyLimit::max(
                    state.settings.api.create_app_max_size,
                )),
            )
            .route("/api/v1/authenticated/tasks", get(task_list_handler))
            .route(
                "/api/v1/authenticated/task/{uuid}",
                get(task_detail_handler),
            )
            .route(
                "/api/v1/authenticated/validate-token",
                post(validate_token_handler),
            )
            .route("/api/v1/authenticated/logout", post(logout_handler))
            .route(
                "/api/v1/authenticated/oauth/device/approve",
                post(approve_device_handler),
            )
            .route(
                "/api/v1/authenticated/oauth/device/deny",
                post(deny_device_handler),
            )
            .route(
                "/api/v1/authenticated/oauth/device/{user_code}",
                get(get_device_handler),
            )
            .route(
                "/api/v1/authenticated/oauth/consent/{id}",
                get(get_consent_handler),
            )
            .route(
                "/api/v1/authenticated/oauth/consent/{id}/approve",
                post(approve_consent_handler),
            )
            .route(
                "/api/v1/authenticated/oauth/consent/{id}/deny",
                post(deny_consent_handler),
            )
            .route(
                "/api/v1/authenticated/connections",
                get(list_connections_handler),
            )
            .route(
                "/api/v1/authenticated/connections/{id}",
                delete(revoke_connection_handler),
            )
            .route("/api/v1/authenticated/blueprints", get(blueprints_handler))
            .route(
                "/api/v1/authenticated/scopes/list",
                get(list_user_scopes_handler),
            )
            .route(
                "/api/v1/authenticated/apps/notify/add",
                post(add_notification_handler),
            )
            .route(
                "/api/v1/authenticated/apps/notify/remove",
                post(remove_notification_handler),
            )
            // This route is still authenticated: it lives under the
            // `/authenticated` group and the `authorization_middleware` applied
            // below populates the `AuthorizationContext` for every route here.
            // Only the *permission* check is deferred to the handler, because it
            // depends on the action's own permission field (ActionRead vs
            // ActionWrite) which isn't known at routing time.
            .route(
                "/api/v1/authenticated/apps/{app_name}/actions",
                post(run_custom_action_handler),
            )
            // Custom action management routes.
            //
            // All management endpoints (create/list/get/delete) require
            // `action_manage`. A single `.layer()` is applied to the whole
            // MethodRouter so it guards every method uniformly. Do NOT chain a
            // second `.layer()` after additional methods here: on a MethodRouter
            // the trailing `.layer()` wraps *all* methods registered so far,
            // which previously left the GET (list) handler guarded only by
            // `View` instead of `action_manage`.
            .route(
                "/api/v1/authenticated/apps/{app_name}/custom-actions",
                post(create_custom_action_handler)
                    .get(list_custom_actions_handler)
                    .layer(middleware::from_fn_with_state(
                        state.clone(),
                        require_permission(Permission::ActionManage),
                    )),
            )
            .route(
                "/api/v1/authenticated/apps/{app_name}/custom-actions/{action_name}",
                get(get_custom_action_handler)
                    .delete(delete_custom_action_handler)
                    .layer(middleware::from_fn_with_state(
                        state.clone(),
                        require_permission(Permission::ActionManage),
                    )),
            )
            // File transfer endpoints (download + upload) for a service's
            // container. GET requires `shell` (reading container files is
            // shell-level access), PUT requires `manage`. The permission
            // middleware is attached per method via `route_layer` and then
            // merged, so that one method's check does NOT also wrap the other.
            .route(
                "/api/v1/apps/{app_id}/services/{service}/files",
                get(download_files_handler)
                    .route_layer(middleware::from_fn_with_state(
                        state.clone(),
                        require_permission(Permission::Shell),
                    ))
                    .merge(
                        put(upload_files_handler)
                            .route_layer(middleware::from_fn_with_state(
                                state.clone(),
                                require_permission(Permission::Manage),
                            ))
                            // Disable the default 2 MiB request body limit for
                            // uploads; transfers are bounded by
                            // `Settings.files.max_transfer_size` via a counting
                            // stream wrapper.
                            .route_layer(DefaultBodyLimit::disable()),
                    ),
            )
            // Admin API routes - require AdminRead/AdminWrite permissions
            // Each method gets its own permission via `route_layer` + `merge`
            // so that a later `.layer()` cannot wrap an earlier method (which
            // would, e.g., make a GET require AdminWrite in addition to
            // AdminRead and lock out read-only admins).
            .route(
                "/api/v1/authenticated/admin/scopes",
                get(list_scopes_handler)
                    .route_layer(middleware::from_fn_with_state(
                        state.clone(),
                        require_permission(Permission::AdminRead),
                    ))
                    .merge(
                        post(create_scope_handler).route_layer(middleware::from_fn_with_state(
                            state.clone(),
                            require_permission(Permission::AdminWrite),
                        )),
                    ),
            )
            .route(
                "/api/v1/authenticated/admin/roles",
                get(list_roles_handler)
                    .route_layer(middleware::from_fn_with_state(
                        state.clone(),
                        require_permission(Permission::AdminRead),
                    ))
                    .merge(
                        post(create_role_handler).route_layer(middleware::from_fn_with_state(
                            state.clone(),
                            require_permission(Permission::AdminWrite),
                        )),
                    ),
            )
            .route(
                "/api/v1/authenticated/admin/assignments",
                get(list_assignments_handler)
                    .route_layer(middleware::from_fn_with_state(
                        state.clone(),
                        require_permission(Permission::AdminRead),
                    ))
                    .merge(
                        // POST and DELETE share the AdminWrite requirement.
                        post(create_assignment_handler)
                            .delete(remove_assignment_handler)
                            .route_layer(middleware::from_fn_with_state(
                                state.clone(),
                                require_permission(Permission::AdminWrite),
                            )),
                    ),
            )
            .route(
                "/api/v1/authenticated/admin/permissions",
                get(list_available_permissions_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::AdminRead),
                )),
            )
            .route(
                "/api/v1/authenticated/admin/permissions/test",
                post(test_permission_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::AdminRead),
                )),
            )
            .route(
                "/api/v1/authenticated/admin/permissions/user/{user_id}",
                get(get_user_permissions_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::AdminRead),
                )),
            )
            // Admin custom action approval routes
            .route(
                "/api/v1/authenticated/admin/actions/pending",
                get(list_pending_actions_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::ActionApprove),
                )),
            )
            .route(
                "/api/v1/authenticated/admin/actions/{app_name}/{action_name}",
                get(get_action_details_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::ActionApprove),
                )),
            )
            .route(
                "/api/v1/authenticated/admin/actions/{app_name}/{action_name}/approve",
                post(approve_action_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::ActionApprove),
                )),
            )
            .route(
                "/api/v1/authenticated/admin/actions/{app_name}/{action_name}/reject",
                post(reject_action_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::ActionApprove),
                )),
            )
            .route(
                "/api/v1/authenticated/admin/actions/{app_name}/{action_name}/revoke",
                post(revoke_action_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::ActionApprove),
                )),
            )
            // Apply authorization middleware to all authenticated routes
            .route_layer(middleware::from_fn_with_state(
                state.clone(),
                authorization_middleware,
            ))
            .route_layer(middleware::from_fn_with_state(state.clone(), auth));

        // Apply rate limiting to authenticated endpoints if enabled
        if rate_limit_config.enabled && rate_limit_config.authenticated.is_enabled() {
            tracing::info!(
                "Rate limiting enabled for authenticated endpoints: {} req/min, burst: {}",
                rate_limit_config.authenticated.requests_per_minute,
                rate_limit_config.authenticated.burst_size
            );
            authenticated_router = authenticated_router
                .layer(create_authenticated_limiter(
                    &rate_limit_config.authenticated,
                ))
                .layer(
                    super::rate_limiting::middleware::RateLimitMetricsLayer::new("authenticated"),
                );
        }

        // Build login router (public auth tier)
        let mut login_router = Router::new()
            .route("/api/v1/login", post(login_handler))
            .with_state(state.clone());

        // Apply rate limiting to login endpoints if enabled
        if rate_limit_config.enabled && rate_limit_config.public_auth.is_enabled() {
            tracing::info!(
                "Rate limiting enabled for public auth endpoints: {} req/min, burst: {}",
                rate_limit_config.public_auth.requests_per_minute,
                rate_limit_config.public_auth.burst_size
            );
            login_router = login_router
                .layer(create_public_auth_limiter(&rate_limit_config.public_auth))
                .layer(super::rate_limiting::middleware::RateLimitMetricsLayer::new("public_auth"));
        }

        // Build OAuth router
        let mut oauth_router = Router::new()
            .route("/oauth/device", post(start_device_flow))
            .route("/oauth/device/token", post(device_token))
            .route("/oauth/authorize", get(start_authorization_flow))
            .route("/api/oauth/callback", get(handle_oauth_callback))
            .route("/oauth/exchange", post(exchange_session_for_token))
            .with_state(state.clone());

        // Apply rate limiting to OAuth endpoints if enabled
        if rate_limit_config.enabled && rate_limit_config.oauth.is_enabled() {
            tracing::info!(
                "Rate limiting enabled for OAuth endpoints: {} req/min, burst: {}",
                rate_limit_config.oauth.requests_per_minute,
                rate_limit_config.oauth.burst_size
            );
            oauth_router = oauth_router
                .layer(create_oauth_limiter(&rate_limit_config.oauth))
                .layer(super::rate_limiting::middleware::RateLimitMetricsLayer::new("oauth"));
        }

        // Build unprotected public router (no rate limiting)
        let public_router = Router::new()
            .route("/api/v1/health", get(health_checker_handler))
            .route("/api/v1/info", get(info_handler))
            .route("/robots.txt", get(robots_txt_handler))
            .route("/ws", get(ws_handler))
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api.clone()))
            .merge(Redoc::with_url("/redoc", api.clone()))
            .merge(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
            .with_state(state.clone());

        // Serve embedded frontend files, or redirect to landing page for stopped apps
        tracing::info!("Serving embedded frontend files with landing page support");
        let router = Router::new()
            .merge(authenticated_router)
            .merge(login_router)
            .merge(oauth_router)
            .merge(public_router)
            .fallback(landing_or_frontend_handler)
            .with_state(state.clone());

        // The MCP authorization server (metadata, /oauth/mcp/*, /mcp), in
        // oauth mode only.
        let router = match crate::oauth::mcp::router(&state) {
            Some(mcp) => router.merge(mcp),
            None => router,
        };

        // Keep search engines from indexing the Scotty server itself (UI + API).
        router.layer(middleware::from_fn_with_state(state, add_noindex_header))
    }
}

/// `/robots.txt` for Scotty's own host. A hosted app's domain also reaches
/// Scotty through the load balancer's fallback; those requests go to the
/// landing handler, which answers according to the app's robots setting.
async fn robots_txt_handler(
    State(state): State<SharedAppState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if !is_scotty_request(&state, &headers) {
        return landing_or_frontend_handler(State(state), headers, uri)
            .await
            .into_response();
    }
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        ROBOTS_TXT,
    )
        .into_response()
}

/// Marks Scotty's own responses `noindex`. Responses for hosted apps' domains
/// (landing redirects, 503s, `/robots.txt`) get the app's own robots setting
/// from the landing handler instead.
async fn add_noindex_header(
    State(state): State<SharedAppState>,
    req: Request,
    next: Next,
) -> Response {
    let ours = is_scotty_request(&state, req.headers());
    let mut res = next.run(req).await;
    if ours {
        res.headers_mut().insert(
            HeaderName::from_static("x-robots-tag"),
            HeaderValue::from_static("noindex, nofollow"),
        );
    }
    res
}

#[cfg(test)]
mod robots_tests {
    use crate::api::router::ApiRoutes;
    use crate::api::test_utils::create_test_app_state_with_settings;
    use axum_test::TestServer;
    use scotty_core::apps::app_data::{
        AppData, AppSettings, AppStatus, ContainerState, ServicePortMapping,
    };

    const SCOTTY: &str = "scotty.example.com";

    fn settings() -> crate::settings::config::Settings {
        config::Config::builder()
            .add_source(config::File::with_name("tests/test_bearer_auth"))
            .set_override("api.base_url", format!("http://{SCOTTY}"))
            .unwrap()
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap()
    }

    async fn server() -> TestServer {
        TestServer::new(ApiRoutes::create(
            create_test_app_state_with_settings(settings(), None).await,
        ))
    }

    fn noindex(res: &axum_test::TestResponse) -> bool {
        res.maybe_header("x-robots-tag")
            .is_some_and(|v| v == "noindex, nofollow")
    }

    #[tokio::test]
    async fn scotty_host_is_noindex_everywhere() {
        let server = server().await;
        let res = server.get("/robots.txt").add_header("host", SCOTTY).await;
        res.assert_status_ok();
        res.assert_text("User-agent: *\nDisallow: /\n");
        assert_eq!(res.header("content-type"), "text/plain; charset=utf-8");
        assert!(noindex(&res));

        for path in [
            "/api/v1/health",                  // public API
            "/api/v1/authenticated/apps/list", // 401 without a token
            "/some/spa/route",                 // frontend fallback
        ] {
            let res = server.get(path).add_header("host", SCOTTY).await;
            assert!(noindex(&res), "{path}: {}", res.status_code());
        }
    }

    use crate::docker::loadbalancer::ROBOTS_HEADER_VALUE as APP_ROBOTS;

    /// Server with two apps: `closed` disallows robots, `open` allows them.
    async fn server_with_apps(status: AppStatus) -> TestServer {
        let state = create_test_app_state_with_settings(settings(), None).await;
        for (name, disallow_robots) in [("closed", true), ("open", false)] {
            state
                .apps
                .add_app(AppData {
                    name: name.to_string(),
                    status: status.clone(),
                    settings: Some(AppSettings {
                        domain: format!("{name}.apps.example.com"),
                        disallow_robots,
                        public_services: vec![ServicePortMapping {
                            service: "web".to_string(),
                            port: 80,
                            domains: vec![],
                        }],
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await
                .unwrap();
        }
        // An app Scotty didn't create: no settings, domain from its container.
        state
            .apps
            .add_app(AppData {
                name: "unmanaged".to_string(),
                status: status.clone(),
                services: vec![ContainerState {
                    service: "web".to_string(),
                    domains: vec!["unmanaged.apps.example.com".to_string()],
                    ..Default::default()
                }],
                ..Default::default()
            })
            .await
            .unwrap();
        TestServer::new(ApiRoutes::create(state))
    }

    fn robots_header(res: &axum_test::TestResponse) -> Option<String> {
        res.maybe_header("x-robots-tag")
            .map(|v| v.to_str().unwrap().to_string())
    }

    #[tokio::test]
    async fn stopped_app_follows_its_robots_setting() {
        let server = server_with_apps(AppStatus::Stopped).await;

        let res = server
            .get("/")
            .add_header("host", "web.closed.apps.example.com")
            .await;
        res.assert_status(axum::http::StatusCode::FOUND);
        assert_eq!(robots_header(&res).as_deref(), Some(APP_ROBOTS));

        let res = server
            .get("/robots.txt")
            .add_header("host", "web.closed.apps.example.com")
            .await;
        res.assert_status_ok();
        res.assert_text("User-agent: *\nDisallow: /\n");
        assert_eq!(robots_header(&res).as_deref(), Some(APP_ROBOTS));
        assert!(res
            .header("cache-control")
            .to_str()
            .unwrap()
            .contains("no-store"));
        assert!(!noindex(&res));

        // Other methods never reach the fallback: the route answers 405.
        let res = server
            .post("/robots.txt")
            .add_header("host", "web.closed.apps.example.com")
            .await;
        res.assert_status(axum::http::StatusCode::METHOD_NOT_ALLOWED);

        let res = server
            .get("/")
            .add_header("host", "web.open.apps.example.com")
            .await;
        res.assert_status(axum::http::StatusCode::FOUND);
        assert_eq!(robots_header(&res), None);

        let res = server
            .get("/robots.txt")
            .add_header("host", "web.open.apps.example.com")
            .await;
        res.assert_status(axum::http::StatusCode::FOUND);
        assert_eq!(robots_header(&res), None);
    }

    #[tokio::test]
    async fn unmanaged_app_gets_no_robots_signal() {
        let server = server_with_apps(AppStatus::Stopped).await;
        for path in ["/", "/robots.txt"] {
            let res = server
                .get(path)
                .add_header("host", "unmanaged.apps.example.com")
                .await;
            res.assert_status(axum::http::StatusCode::FOUND);
            assert_eq!(robots_header(&res), None, "{path}");
        }
    }

    #[tokio::test]
    async fn starting_app_503_follows_its_robots_setting() {
        let server = server_with_apps(AppStatus::Running).await;
        let res = server
            .get("/")
            .add_header("host", "web.closed.apps.example.com")
            .await;
        res.assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(robots_header(&res).as_deref(), Some(APP_ROBOTS));

        // robots.txt is only answered directly for stopped apps.
        let res = server
            .get("/robots.txt")
            .add_header("host", "web.closed.apps.example.com")
            .await;
        res.assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(robots_header(&res).as_deref(), Some(APP_ROBOTS));

        let res = server
            .get("/")
            .add_header("host", "web.open.apps.example.com")
            .await;
        res.assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(robots_header(&res), None);
    }

    #[tokio::test]
    async fn app_hosts_keep_their_own_robots_setting() {
        let server = server().await;
        // A hosted app's domain reaching Scotty via the load balancer fallback.
        let res = server
            .get("/robots.txt")
            .add_header("host", "shop.apps.example.com")
            .await;
        assert_ne!(res.text(), "User-agent: *\nDisallow: /\n");
        assert!(!noindex(&res));

        let res = server
            .get("/")
            .add_header("host", "shop.apps.example.com")
            .await;
        assert!(!noindex(&res));
    }
}
