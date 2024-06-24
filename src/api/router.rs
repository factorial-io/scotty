use axum::routing::get;
use axum::Router;
use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

use crate::api::handlers::health::__path_health_checker_handler;

use crate::api::handlers::health::health_checker_handler;
use crate::app_state::SharedAppState;

#[derive(OpenApi)]
#[openapi(
    paths(
        health_checker_handler,
    ),
    components(
        schemas( )
    ),
    tags(
        (name = "yafbds-service", description = "yafbds api")
    )
)]
struct ApiDoc;
pub struct ApiRoutes;

impl ApiRoutes {
    pub fn create(state: SharedAppState) -> Router {
        let router = Router::new()
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
            .merge(Redoc::with_url("/redoc", ApiDoc::openapi()))
            .merge(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
            .route("/api/v1/health", get(health_checker_handler))
            .with_state(state);

        router
    }
}
