use axum::routing::get;
use axum::Router;
use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

use crate::api::handlers::apps::list::__path_list_apps_handler;
use crate::api::handlers::apps::list::list_apps_handler;
use crate::api::handlers::apps::run::__path_info_app_handler;
use crate::api::handlers::apps::run::__path_purge_app_handler;
use crate::api::handlers::apps::run::__path_rebuild_app_handler;
use crate::api::handlers::apps::run::__path_run_app_handler;
use crate::api::handlers::apps::run::__path_stop_app_handler;
use crate::api::handlers::health::__path_health_checker_handler;
use crate::api::handlers::health::health_checker_handler;
use crate::api::handlers::tasks::__path_task_detail_handler;
use crate::api::ws::ws_handler;
use crate::app_state::SharedAppState;
use crate::apps::app_data::AppData;
use crate::apps::app_data::AppSettings;
use crate::apps::app_data::AppState;
use crate::apps::app_data::AppTtl;
use crate::apps::app_data::ContainerState;
use crate::apps::app_data::ServicePortMapping;
use crate::apps::shared_app_list::AppDataVec;
use crate::tasks::running_app_context::RunningAppContext;
use crate::tasks::task_details::TaskDetails;

use super::handlers::apps::run::info_app_handler;
use super::handlers::apps::run::purge_app_handler;
use super::handlers::apps::run::rebuild_app_handler;
use super::handlers::apps::run::run_app_handler;
use super::handlers::apps::run::stop_app_handler;
use super::handlers::tasks::task_detail_handler;

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
    ),
    components(
        schemas( AppData, AppDataVec, TaskDetails, ContainerState, AppSettings, AppState, AppTtl, ServicePortMapping, RunningAppContext)
    ),
    tags(
        (name = "yafbds-service", description = "yafbds api")
    )
)]
struct ApiDoc;
pub struct ApiRoutes;

impl ApiRoutes {
    pub fn create(state: SharedAppState) -> Router {
        Router::new()
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
            .merge(Redoc::with_url("/redoc", ApiDoc::openapi()))
            .merge(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
            .route("/api/v1/health", get(health_checker_handler))
            .route("/api/v1/apps/list", get(list_apps_handler))
            .route("/api/v1/apps/run/:app_id", get(run_app_handler))
            .route("/api/v1/apps/stop/:app_id", get(stop_app_handler))
            .route("/api/v1/apps/purge/:app_id", get(purge_app_handler))
            .route("/api/v1/apps/rebuild/:app_id", get(rebuild_app_handler))
            .route("/api/v1/apps/info/:app_id", get(info_app_handler))
            .route("/api/v1/task/:uuid", get(task_detail_handler))
            .route("/ws", get(ws_handler))
            .with_state(state)
    }
}
