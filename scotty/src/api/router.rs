use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{delete, get, post};
use axum::Router;
use scotty_core::apps::app_data::AppData;
use scotty_core::apps::app_data::AppSettings;
use scotty_core::apps::app_data::AppStatus;
use scotty_core::apps::app_data::AppTtl;
use scotty_core::apps::app_data::ContainerState;
use scotty_core::apps::app_data::ServicePortMapping;
use scotty_core::apps::create_app_request::CreateAppRequest;
use scotty_core::apps::file_list::File;
use scotty_core::apps::file_list::FileList;
use scotty_core::apps::shared_app_list::AppDataVec;
use scotty_core::notification_types::AddNotificationRequest;
use scotty_core::notification_types::GitlabContext;
use scotty_core::notification_types::MattermostContext;
use scotty_core::notification_types::NotificationReceiver;
use scotty_core::notification_types::WebhookContext;
use scotty_core::tasks::running_app_context::RunningAppContext;

use utoipa::openapi::security::SecurityScheme;
use utoipa::Modify;
use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

use crate::api::rest::handlers::apps::create::__path_create_app_handler;
use crate::api::rest::handlers::apps::custom_action::__path_run_custom_action_handler;
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
use crate::api::rest::handlers::apps::shell::__path_create_shell_handler;
use crate::api::rest::handlers::apps::shell::__path_resize_tty_handler;
use crate::api::rest::handlers::apps::shell::__path_shell_input_handler;
use crate::api::rest::handlers::apps::shell::__path_terminate_shell_handler;
use crate::api::rest::handlers::health::__path_health_checker_handler;
use crate::api::rest::handlers::info::__path_info_handler;
use crate::api::rest::handlers::login::__path_login_handler;
use crate::api::rest::handlers::login::__path_validate_token_handler;
use crate::oauth::handlers::{
    exchange_session_for_token, handle_oauth_callback, poll_device_token, start_authorization_flow,
    start_device_flow,
};
use crate::oauth::handlers::{AuthorizeQuery, CallbackQuery, DeviceFlowResponse, TokenResponse};
use scotty_core::api::{OAuthConfig, ServerInfo};
use scotty_core::settings::api_server::AuthMode;

use crate::api::rest::handlers::admin::assignments::{
    __path_create_assignment_handler, __path_list_assignments_handler,
    __path_remove_assignment_handler,
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
use crate::api::rest::handlers::health::health_checker_handler;
use crate::api::rest::handlers::scopes::list::__path_list_user_scopes_handler;
use crate::api::rest::handlers::tasks::TaskList;
use crate::api::rest::handlers::tasks::__path_task_detail_handler;
use crate::api::rest::handlers::tasks::__path_task_list_handler;
use crate::api::websocket::client::ws_handler;
use crate::app_state::SharedAppState;
use crate::static_files::serve_embedded_file;
use scotty_core::tasks::task_details::TaskDetails;

use super::basic_auth::auth;
use super::middleware::authorization::{authorization_middleware, require_permission};
use super::rate_limiting::{
    create_authenticated_limiter, create_oauth_limiter, create_public_auth_limiter,
};
use super::rest::handlers::admin::assignments::{
    create_assignment_handler, list_assignments_handler, remove_assignment_handler,
};
use super::rest::handlers::admin::permissions::{
    get_user_permissions_handler, list_available_permissions_handler, test_permission_handler,
};
use super::rest::handlers::admin::roles::{create_role_handler, list_roles_handler};
use super::rest::handlers::admin::scopes::{create_scope_handler, list_scopes_handler};
use super::rest::handlers::apps::create::create_app_handler;
use super::rest::handlers::apps::custom_action::run_custom_action_handler;
use super::rest::handlers::apps::notify::add_notification_handler;
use super::rest::handlers::apps::notify::remove_notification_handler;
use super::rest::handlers::apps::run::adopt_app_handler;
use super::rest::handlers::apps::run::destroy_app_handler;
use super::rest::handlers::apps::run::info_app_handler;
use super::rest::handlers::apps::run::purge_app_handler;
use super::rest::handlers::apps::run::rebuild_app_handler;
use super::rest::handlers::apps::run::run_app_handler;
use super::rest::handlers::apps::run::stop_app_handler;
use super::rest::handlers::apps::shell::{
    create_shell_handler, resize_tty_handler, shell_input_handler, terminate_shell_handler,
};
use super::rest::handlers::blueprints::blueprints_handler;
use super::rest::handlers::info::info_handler;
use super::rest::handlers::login::login_handler;
use super::rest::handlers::login::validate_token_handler;
use super::rest::handlers::scopes::list::{
    list_user_scopes_handler, ScopeInfo, UserScopesResponse,
};
use super::rest::handlers::tasks::task_detail_handler;
use super::rest::handlers::tasks::task_list_handler;
use crate::api::rest::handlers::apps::shell::{
    CreateShellRequest, CreateShellResponse, ResizeTtyRequest, ResizeTtyResponse,
    ShellInputRequest, ShellInputResponse, TerminateShellResponse,
};
use crate::docker::services::shell::ShellServiceError;
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
        login_handler,
        info_handler,
        blueprints_handler,
        list_user_scopes_handler,
        add_notification_handler,
        remove_notification_handler,
        adopt_app_handler,
        run_custom_action_handler,
        // Shell endpoints
        create_shell_handler,
        shell_input_handler,
        resize_tty_handler,
        terminate_shell_handler,
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
    ),
    components(
        schemas(
            GitlabContext, WebhookContext, MattermostContext, NotificationReceiver,
            AddNotificationRequest, TaskList, File, FileList, CreateAppRequest,
            AppData, AppDataVec, TaskDetails, ContainerState, AppSettings,
            AppStatus, AppTtl, ServicePortMapping, RunningAppContext,
            OAuthConfig, ServerInfo, AuthMode, DeviceFlowResponse, TokenResponse, AuthorizeQuery, CallbackQuery,
            ScopeInfo, UserScopesResponse,
            // Admin API schemas
            AdminScopeInfo, ScopesListResponse, CreateScopeRequest, CreateScopeResponse,
            RoleInfo, RolesListResponse, CreateRoleRequest, CreateRoleResponse,
            AssignmentInfo, AssignmentsListResponse, CreateAssignmentRequest, CreateAssignmentResponse,
            RemoveAssignmentRequest, RemoveAssignmentResponse, Assignment,
            TestPermissionRequest, TestPermissionResponse, UserPermissionsResponse, AvailablePermissionsResponse,
            // Shell API schemas
            CreateShellRequest, CreateShellResponse, ShellInputRequest, ShellInputResponse,
            ResizeTtyRequest, ResizeTtyResponse, TerminateShellResponse, ShellServiceError
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
            .route(
                "/api/v1/authenticated/apps/{app_name}/actions",
                post(run_custom_action_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::Manage),
                )),
            )
            // Shell API routes
            .route(
                "/api/v1/authenticated/apps/{app_id}/services/{service_name}/shell",
                post(create_shell_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::Manage),
                )),
            )
            .route(
                "/api/v1/authenticated/shell/sessions/{session_id}/input",
                post(shell_input_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::Manage),
                )),
            )
            .route(
                "/api/v1/authenticated/shell/sessions/{session_id}/resize",
                post(resize_tty_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::Manage),
                )),
            )
            .route(
                "/api/v1/authenticated/shell/sessions/{session_id}",
                delete(terminate_shell_handler).layer(middleware::from_fn_with_state(
                    state.clone(),
                    require_permission(Permission::Manage),
                )),
            )
            // Admin API routes - require AdminRead/AdminWrite permissions
            .route(
                "/api/v1/authenticated/admin/scopes",
                get(list_scopes_handler)
                    .layer(middleware::from_fn_with_state(
                        state.clone(),
                        require_permission(Permission::AdminRead),
                    ))
                    .post(create_scope_handler)
                    .layer(middleware::from_fn_with_state(
                        state.clone(),
                        require_permission(Permission::AdminWrite),
                    )),
            )
            .route(
                "/api/v1/authenticated/admin/roles",
                get(list_roles_handler)
                    .layer(middleware::from_fn_with_state(
                        state.clone(),
                        require_permission(Permission::AdminRead),
                    ))
                    .post(create_role_handler)
                    .layer(middleware::from_fn_with_state(
                        state.clone(),
                        require_permission(Permission::AdminWrite),
                    )),
            )
            .route(
                "/api/v1/authenticated/admin/assignments",
                get(list_assignments_handler)
                    .layer(middleware::from_fn_with_state(
                        state.clone(),
                        require_permission(Permission::AdminRead),
                    ))
                    .post(create_assignment_handler)
                    .layer(middleware::from_fn_with_state(
                        state.clone(),
                        require_permission(Permission::AdminWrite),
                    ))
                    .delete(remove_assignment_handler)
                    .layer(middleware::from_fn_with_state(
                        state.clone(),
                        require_permission(Permission::AdminWrite),
                    )),
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
            .route("/oauth/device/token", post(poll_device_token))
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
            .route("/ws", get(ws_handler))
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api.clone()))
            .merge(Redoc::with_url("/redoc", api.clone()))
            .merge(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
            .with_state(state.clone());

        let router = Router::new()
            .merge(authenticated_router)
            .merge(login_router)
            .merge(oauth_router)
            .merge(public_router)
            .with_state(state.clone());

        // Always use embedded frontend files
        tracing::info!("Serving embedded frontend files");
        router.fallback(|uri: axum::http::Uri| async move { serve_embedded_file(uri).await })
    }
}
