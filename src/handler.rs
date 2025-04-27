pub mod area;
pub mod auth;
pub mod project;
pub mod tag;
pub mod task;

use axum::{Json, response::IntoResponse};

pub async fn health_check_handler() -> impl IntoResponse {
    const MESSAGE: &str = "Todo List API Services";

    let json_response = serde_json::json!({
        "status": "ok",
        "message": MESSAGE
    });

    Json(json_response)
}

// TEST: healthcheck handler
