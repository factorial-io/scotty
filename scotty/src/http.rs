use axum::http::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    HeaderValue, Method,
};
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::{api::router::ApiRoutes, app_state::SharedAppState};

pub async fn setup_http_server(
    app_state: SharedAppState,
    bind_address: &str,
) -> anyhow::Result<tokio::task::JoinHandle<anyhow::Result<()>>> {
    let cors = CorsLayer::new()
        .allow_origin("*".parse::<HeaderValue>().unwrap())
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        // .allow_credentials(true)
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);

    let app = ApiRoutes::create(app_state.clone())
        .layer(cors)
        .layer(OtelInResponseLayer)
        .layer(OtelAxumLayer::default());

    println!("🚀 API-Server starting at {}", &bind_address);
    let listener = tokio::net::TcpListener::bind(&bind_address).await.unwrap();

    let stop_flag = app_state.clone().stop_flag.clone();
    let handle = tokio::spawn({
        let stop_flag = stop_flag.clone();
        async move {
            info!("Starting HTTP server");
            axum::serve(listener, app)
                .with_graceful_shutdown({
                    let stop_flag = stop_flag.clone();
                    async move {
                        stop_flag.wait().await;
                        info!("Stop flag was set, shutting down HTTP server gracefully");
                    }
                })
                .await
                .unwrap();
            info!("HTTP server is down");
            Ok(())
        }
    });

    Ok(handle)
}
