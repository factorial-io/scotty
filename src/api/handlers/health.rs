use axum::{response::IntoResponse, Json};
#[utoipa::path(
    get,
    path = "/api/health",
    responses(
    (status = 200, description = "Health check")
    )
)]
pub async fn health_checker_handler() -> impl IntoResponse {
    const MESSAGE: &str = "WorkerBridge is running!";

    let json_response = serde_json::json!({
        "status": "success",
        "message": MESSAGE
    });

    Json(json_response)
}
