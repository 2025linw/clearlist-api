mod tag;
mod task;

use std::sync::Arc;

use axum::{
    Router,
    handler::Handler,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
};
use governor::{clock::QuantaInstant, middleware::NoOpMiddleware};
use serde_json::json;
use tower_governor::{
    GovernorLayer,
    governor::{GovernorConfig, GovernorConfigBuilder},
    key_extractor::SmartIpKeyExtractor,
};

use crate::{
    AppState,
    response::{OK, Response},
};

/// Handler for API health check
///
/// Responds with OK (200) and a message
pub async fn health_check_handler() -> impl IntoResponse {
    const MESSAGE: &str = "Todo List API Services";

    Response::with_msg(StatusCode::OK, OK, MESSAGE)
        .add_kv("version", json!(env!("CARGO_PKG_VERSION")))
}

/// Create a rate limiter
fn create_rate_limiter(
    num_requests: u32,
    refresh_rate: u64,
) -> GovernorConfig<SmartIpKeyExtractor, NoOpMiddleware<QuantaInstant>> {
    GovernorConfigBuilder::default()
        .key_extractor(SmartIpKeyExtractor)
        .burst_size(num_requests)
        .per_second(refresh_rate)
        .finish()
        .unwrap()
}

/// Create API router for project
pub fn create_api_router() -> Router<AppState> {
    let task_routes = create_resource_router(
        task::create_handler,
        task::retrieve_handler,
        task::update_handler,
        task::delete_handler,
        task::query_handler,
    )
    .nest(
        "/{task_id}/tags",
        Router::new()
            .route("/", get(task::tag::query_handler))
            .route(
                "/{tag_id}",
                post(task::tag::create_handler).delete(task::tag::delete_handler),
            )
            .route("/", put(task::tag::update_handler)),
    );
    let tag_routes = create_resource_router(
        tag::create_handler,
        tag::retrieve_handler,
        tag::update_handler,
        tag::delete_handler,
        tag::query_handler,
    );

    let api_routes = Router::new()
        .nest("/tasks", task_routes)
        .nest("/tags", tag_routes)
        .layer(GovernorLayer {
            config: Arc::new(create_rate_limiter(8, 1)),
        });

    Router::new().merge(api_routes)
}

fn create_resource_router<C, R, U, D, Q, T1, T2, T3, T4, T5>(
    create_handler: C,
    retrieve_handler: R,
    update_handler: U,
    delete_handler: D,
    query_handler: Q,
) -> Router<AppState>
where
    C: Handler<T1, AppState>,
    R: Handler<T2, AppState>,
    U: Handler<T3, AppState>,
    D: Handler<T4, AppState>,
    Q: Handler<T5, AppState>,
    T1: 'static,
    T2: 'static,
    T3: 'static,
    T4: 'static,
    T5: 'static,
{
    Router::new()
        .route("/", get(query_handler))
        .route("/", post(create_handler))
        .route(
            "/{id}",
            get(retrieve_handler)
                .put(update_handler)
                .delete(delete_handler),
        )
}
