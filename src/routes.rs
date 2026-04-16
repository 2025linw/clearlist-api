//! # Routes Module
//!
//! This module contains all routing functions and handlers

pub mod models;

mod error;
mod util;

mod tag;
mod task;

pub use error::Error;

use std::sync::Arc;

use axum::{
    Router,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch},
};
use serde_json::json;
use tower_governor::GovernorLayer;

use crate::{AppState, response::Response};

/// Create API router for all resources
///
/// Router is rate limited to 8 requests refreshing 1 every 1 second
pub fn create_api_router() -> Router<AppState> {
    // TOOD: replace create_resource_router with the actual ends
    let task_routes = Router::new()
        .route("/", get(task::query_handler).post(task::create_handler))
        .route(
            "/{task_id}",
            get(task::retrieve_handler)
                .put(task::update_handler)
                .delete(task::delete_handler),
        )
        .route("/{task_id}/restore", patch(task::restore_handler))
        .route("/{task_id}/complete", patch(task::complete_handler))
        .nest(
            "/{task_id}/tags",
            Router::new()
                .route(
                    "/",
                    get(task::tag::query_handler).put(task::tag::update_handler),
                )
                .route(
                    "/{tag_id}",
                    patch(task::tag::append_handler).delete(task::tag::delete_handler),
                ),
        );

    let tag_routes = Router::new()
        .route("/", get(tag::query_handler).post(tag::create_handler))
        .route(
            "/{tag_id}",
            get(tag::retrieve_handler)
                .put(tag::update_handler)
                .delete(tag::delete_handler),
        );

    Router::new()
        .nest("/tasks", task_routes)
        .nest("/tags", tag_routes)
        .layer(GovernorLayer {
            config: Arc::new(util::create_rate_limiter(8, 1)),
        })
}

/// Handler for API health check
///
/// Responds with OK (200) and a message
pub async fn health_check_handler() -> impl IntoResponse {
    const MESSAGE: &str = "Todo List API Services";

    Response::new(StatusCode::OK)
        .message(MESSAGE)
        .add_kv("version", json!(env!("CARGO_PKG_VERSION")))
}
