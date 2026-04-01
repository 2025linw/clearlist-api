mod com;
mod error;
mod response;
mod util;

mod db;
mod routes;

pub use db::DatabaseConn;

use std::env;

use axum::{
    Router,
    http::{HeaderValue, Method, header},
    routing::get,
};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    db: DatabaseConn,
}

impl AppState {
    pub fn init(conn: DatabaseConn) -> Self {
        Self { db: conn }
    }
}

pub fn create_app(app_state: AppState) -> Router {
    let origins: Vec<HeaderValue> = env::var("ALLOWED_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(|url| url.parse().unwrap())
        .collect();
    let headers = [
        header::CONTENT_TYPE,
        // header::AUTHORIZATION,
    ];

    let cors = CorsLayer::new()
        .allow_credentials(true)
        .allow_origin(origins)
        .allow_headers(headers)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE]);

    Router::new()
        .route("/health", get(routes::health_check_handler))
        .nest("/api", routes::create_api_router())
        .with_state(app_state)
        .layer(ServiceBuilder::new().layer(cors))
}
