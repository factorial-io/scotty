use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::get;
use axum::routing::post;
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
use tower_http::services::ServeDir;
use tower_http::services::ServeFile;
use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

use crate::api::handlers::apps::create::__path_create_app_handler;
use crate::api::handlers::apps::list::__path_list_apps_handler;
use crate::api::handlers::apps::list::list_apps_handler;
use crate::api::handlers::apps::notify::__path_add_notification_handler;
use crate::api::handlers::apps::notify::__path_remove_notification_handler;
use crate::api::handlers::apps::run::__path_adopt_app_handler;
use crate::api::handlers::apps::run::__path_destroy_app_handler;
use crate::api::handlers::apps::run::__path_info_app_handler;
use crate::api::handlers::apps::run::__path_purge_app_handler;
use crate::api::handlers::apps::run::__path_rebuild_app_handler;
use crate::api::handlers::apps::run::__path_run_app_handler;
use crate::api::handlers::apps::run::__path_stop_app_handler;
use crate::api::handlers::health::__path_health_checker_handler;
use crate::api::handlers::info::__path_info_handler;
use crate::api::handlers::login::__path_login_handler;
use crate::api::handlers::login::__path_validate_token_handler;

use crate::api::handlers::blueprints::__path_blueprints_handler;
use crate::api::handlers::health::health_checker_handler;
use crate::api::handlers::tasks::TaskList;
use crate::api::handlers::tasks::__path_task_detail_handler;
use crate::api::handlers::tasks::__path_task_list_handler;
use crate::api::ws::ws_handler;
use crate::app_state::SharedAppState;
use scotty_core::tasks::task_details::TaskDetails;

use super::basic_auth::auth;
use super::handlers::apps::create::create_app_handler;
use super::handlers::apps::notify::add_notification_handler;
use super::handlers::apps::notify::remove_notification_handler;
use super::handlers::apps::run::adopt_app_handler;
use super::handlers::apps::run::destroy_app_handler;
use super::handlers::apps::run::info_app_handler;
use super::handlers::apps::run::purge_app_handler;
use super::handlers::apps::run::rebuild_app_handler;
use super::handlers::apps::run::run_app_handler;
use super::handlers::apps::run::stop_app_handler;
use super::handlers::blueprints::blueprints_handler;
use super::handlers::info::info_handler;
use super::handlers::login::login_handler;
use super::handlers::login::validate_token_handler;
use super::handlers::tasks::task_detail_handler;
use super::handlers::tasks::task_list_handler;

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
        add_notification_handler,
        remove_notification_handler,
        adopt_app_handler,
    ),
    components(
        schemas(
            GitlabContext, WebhookContext, MattermostContext, NotificationReceiver,
            AddNotificationRequest, TaskList, File, FileList, CreateAppRequest,
            AppData, AppDataVec, TaskDetails, ContainerState, AppSettings,
            AppStatus, AppTtl, ServicePortMapping, RunningAppContext
        )
    ),
    tags(
        (name = "scotty-service", description = "scotty api")
    )
)]
struct ApiDoc;
pub struct ApiRoutes;

impl ApiRoutes {
    pub fn create(state: SharedAppState) -> Router {
        let frontend_directory = state.settings.frontend_directory.clone();
        let api = ApiDoc::openapi();
        let protected_router = Router::new()
            .route("/api/v1/apps/list", get(list_apps_handler))
            .route("/api/v1/apps/run/{app_id}", get(run_app_handler))
            .route("/api/v1/apps/stop/{app_id}", get(stop_app_handler))
            .route("/api/v1/apps/purge/{app_id}", get(purge_app_handler))
            .route("/api/v1/apps/rebuild/{app_id}", get(rebuild_app_handler))
            .route("/api/v1/apps/info/{app_id}", get(info_app_handler))
            .route("/api/v1/apps/destroy/{app_id}", get(destroy_app_handler))
            .route("/api/v1/apps/adopt/{app_id}", get(adopt_app_handler))
            .route(
                "/api/v1/apps/create",
                post(create_app_handler).layer(DefaultBodyLimit::max(
                    state.settings.api.create_app_max_size,
                )),
            )
            .route("/api/v1/tasks", get(task_list_handler))
            .route("/api/v1/task/{uuid}", get(task_detail_handler))
            .route("/api/v1/validate-token", post(validate_token_handler))
            .route("/api/v1/blueprints", get(blueprints_handler))
            .route("/api/v1/apps/notify/add", post(add_notification_handler))
            .route(
                "/api/v1/apps/notify/remove",
                post(remove_notification_handler),
            )
            .route_layer(middleware::from_fn_with_state(state.clone(), auth));

        let public_router = Router::new()
            .route("/api/v1/login", post(login_handler))
            .route("/api/v1/health", get(health_checker_handler))
            .route("/api/v1/info", get(info_handler))
            .route("/ws", get(ws_handler))
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api.clone()))
            .merge(Redoc::with_url("/redoc", api.clone()))
            .merge(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
            .with_state(state.clone());

        let router = Router::new()
            .merge(protected_router)
            .merge(public_router)
            .with_state(state.clone());

        match &frontend_directory {
            None => {
                tracing::info!("No frontend directory provided, serving only the API.");
                router // Return router directly
            }
            Some(frontend_directory) => {
                tracing::info!("Using {} to serve the frontend ui.", frontend_directory);
                let serve_dir = ServeDir::new(frontend_directory).not_found_service(
                    ServeFile::new(format!("{}/index.html", &frontend_directory)),
                );

                let frontend_router = Router::new().fallback_service(serve_dir);

                router.merge(frontend_router)
            }
        }
    }
}
