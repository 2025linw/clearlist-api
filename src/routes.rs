mod area;
mod auth;
mod project;
mod tag;
mod task;
mod user;

use std::sync::Arc;

use axum::{
    Router,
    handler::Handler,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
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
        .per_second(refresh_rate)
        .burst_size(num_requests)
        .finish()
        .unwrap()
}

/// Create API router for project
pub fn create_api_router() -> Router<AppState> {
    let auth_routes = Router::new()
        .route("/register", post(auth::registration_handler))
        .route("/login", post(auth::login_handler))
        .route("/refresh", post(auth::refresh_handler))
        // .route("/reset", post(auth::password_reset_handler))
        .layer(GovernorLayer {
            config: Arc::new(create_rate_limiter(4, 2)),
        });
    let user_routes = Router::new()
        .route(
            "/{id}",
            get(user::retrieve_handler)
                .patch(user::update_handler)
                .delete(user::delete_handler),
        )
        .layer(GovernorLayer {
            config: Arc::new(create_rate_limiter(4, 2)),
        });

    let task_route = create_resource_router(
        task::create_handler,
        task::retrieve_handler,
        task::update_handler,
        task::delete_handler,
        task::query_handler,
    );

    let project_route = create_resource_router(
        project::create_handler,
        project::retrieve_handler,
        project::update_handler,
        project::delete_handler,
        project::query_handler,
    );

    let area_route = create_resource_router(
        area::create_handler,
        area::retrieve_handler,
        area::update_handler,
        area::delete_handler,
        area::query_handler,
    );

    let tag_route = create_resource_router(
        tag::create_handler,
        tag::retrieve_handler,
        tag::update_handler,
        tag::delete_handler,
        tag::query_handler,
    );

    let api_routes = Router::new()
        .nest("/tasks", task_route)
        .nest("/projects", project_route)
        .nest("/areas", area_route)
        .nest("/tags", tag_route)
        .layer(GovernorLayer {
            config: Arc::new(create_rate_limiter(8, 1)),
        });

    Router::new()
        .nest("/auth", auth_routes)
        .nest("/users", user_routes)
        .merge(api_routes)
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
        .route("/", post(query_handler))
        .route("/create", post(create_handler))
        .route(
            "/{id}",
            get(retrieve_handler)
                .patch(update_handler)
                .delete(delete_handler),
        )
}
